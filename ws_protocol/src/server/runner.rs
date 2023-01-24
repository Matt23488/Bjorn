use std::{
    collections::HashMap,
    net::SocketAddr,
    sync::{Arc, Mutex},
};

use futures_util::{stream::SplitStream, SinkExt, StreamExt, TryStreamExt};
use tokio::{
    net::{TcpListener, TcpStream},
    sync::{mpsc, oneshot},
};
use tokio_tungstenite::WebSocketStream;

use crate::{message::Message, ApiSpecifier, Handshake};

type Tx = mpsc::UnboundedSender<tungstenite::Message>;

pub struct Runner {
    on_cancel: oneshot::Receiver<()>,
    tx_sender: mpsc::UnboundedSender<Tx>,
}

impl Runner {
    pub fn new(on_cancel: oneshot::Receiver<()>, tx_sender: mpsc::UnboundedSender<Tx>) -> Runner {
        Runner {
            on_cancel,
            tx_sender,
        }
    }

    pub async fn run(self, addr: String) {
        tokio::select! {
            _ = tokio::spawn(listen(addr, self.tx_sender)) => {}
            _ = self.on_cancel => {}
        };
    }
}

async fn listen(addr: String, tx_sender: mpsc::UnboundedSender<Tx>) {
    let emitters = Clients::new(Mutex::new(HashMap::new()));
    let handlers = Clients::new(Mutex::new(HashMap::new()));

    let listener = TcpListener::bind(&addr)
        .await
        .expect("Error binding to specified address");
    println!("Listening on: {addr}");

    while let Ok((stream, addr)) = listener.accept().await {
        tokio::task::spawn(handle_connection(
            emitters.clone(),
            handlers.clone(),
            stream,
            addr,
            tx_sender.clone(),
        ));
    }
}

type Clients = Arc<Mutex<HashMap<ApiSpecifier, Vec<Tx>>>>;

async fn handle_connection(
    emitters: Clients,
    handlers: Clients,
    raw_stream: TcpStream,
    addr: SocketAddr,
    tx_sender: mpsc::UnboundedSender<Tx>,
) {
    println!("Incoming TCP connection from: {addr}");

    let ws_stream = tokio_tungstenite::accept_async(raw_stream)
        .await
        .expect("Error during the websocket handshake occurred");

    let (mut outgoing, mut incoming) = ws_stream.split();

    println!("Attempting to send handshake token...");
    if let Err(_) = outgoing.send(Handshake::ServerIdentification.into()).await {
        println!("Couldn't send handshake token");
        return;
    }

    let handshake_response = match incoming.next().await {
        Some(Ok(msg)) => msg,
        val => {
            println!("Couldn't get handshake response: {val:?}");
            return;
        }
    };

    let api_specifier = match Handshake::try_from(handshake_response) {
        Ok(Handshake::ClientIdentification(api_specifier)) => api_specifier,
        Ok(handshake_response) => {
            println!("Invalid handshake response: {handshake_response:?}");
            return;
        }
        Err(e) => {
            println!("Invalid client type: {e}");
            return;
        }
    };

    println!("WebSocket connection established: {api_specifier:?} ({addr})");

    let (tx, mut rx) = mpsc::unbounded_channel();
    tx_sender.send(tx.clone()).unwrap();

    let broadcast_incoming = async {
        match api_specifier {
            ApiSpecifier::Emits(_) => {
                handle_emitter(
                    emitters.clone(),
                    handlers.clone(),
                    api_specifier.clone(),
                    tx.clone(),
                    incoming,
                )
                .await
            }
            ApiSpecifier::Handles(_) => {
                handle_handler(
                    emitters.clone(),
                    handlers.clone(),
                    api_specifier.clone(),
                    tx.clone(),
                    incoming,
                )
                .await
            }
        }
    };

    let receive_from_others = async move {
        loop {
            let message = match rx.recv().await {
                Some(msg) => msg,
                None => break,
            };

            outgoing.send(message).await.unwrap_or_default();
        }
    };

    tokio::select! {
        _ = broadcast_incoming => {}
        _ = tokio::spawn(receive_from_others) => {}
    }

    match api_specifier {
        ApiSpecifier::Emits(_) => {
            if let Some(emitters) = emitters.lock().unwrap().get_mut(&api_specifier) {
                if let Some((index, _)) = emitters
                    .iter()
                    .enumerate()
                    .find(|(_, e)| e.same_channel(&tx))
                {
                    emitters.remove(index);
                }
            }
        }
        ApiSpecifier::Handles(_) => {
            if let Some(handlers) = handlers.lock().unwrap().get_mut(&api_specifier) {
                if let Some((index, _)) = handlers
                    .iter()
                    .enumerate()
                    .find(|(_, h)| h.same_channel(&tx))
                {
                    handlers.remove(index);
                }
            }
        }
    }

    println!("{api_specifier:?} ({addr}) disconnected");
}

async fn handle_emitter(
    emitters: Clients,
    handlers: Clients,
    api_specifier: ApiSpecifier,
    tx: Tx,
    incoming: SplitStream<WebSocketStream<TcpStream>>,
) {
    {
        let mut emitters_lock = emitters.lock().unwrap();

        match emitters_lock.get_mut(&api_specifier) {
            Some(emitters) => {
                emitters.push(tx);
            }
            None => {
                emitters_lock.insert(api_specifier, vec![tx]);
            }
        }
    }

    incoming
        .try_for_each(move |msg| {
            let handlers = handlers.clone();
            async move {
                let ws_message = match Message::try_from(msg.clone()) {
                    Ok(msg) => msg,
                    Err(_) => {
                        eprintln!("Unknown message, ignoring: {msg:?}");
                        return Ok(());
                    }
                };

                let handlers = handlers.lock().unwrap();
                let handlers = match handlers.get(&ws_message.target_api_specifier()) {
                    Some(handler) => handler,
                    None => {
                        println!(
                            "No {:?} client connected.",
                            ws_message.target_api_specifier()
                        );
                        return Ok(());
                    }
                };

                for handler in handlers {
                    handler.send(ws_message.clone().into()).unwrap();
                }

                Ok(())
            }
        })
        .await
        .unwrap_or_default();
}

async fn handle_handler(
    emitters: Clients,
    handlers: Clients,
    api_specifier: ApiSpecifier,
    tx: Tx,
    incoming: SplitStream<WebSocketStream<TcpStream>>,
) {
    {
        let mut handlers_lock = handlers.lock().unwrap();
        match handlers_lock.get_mut(&api_specifier) {
            Some(handlers) => {
                handlers.push(tx);
            }
            None => {
                handlers_lock.insert(api_specifier, vec![tx]);
            }
        }
    }

    incoming
        .try_for_each(move |msg| {
            let emitters = emitters.clone();
            async move {
                let ws_message = match Message::try_from(msg.clone()) {
                    Ok(msg) => msg,
                    Err(_) => {
                        println!("Unknown message, ignoring: {msg:?}");
                        return Ok(());
                    }
                };

                let emitters = emitters.lock().unwrap();
                let emitters = match emitters.get(&ws_message.target_api_specifier()) {
                    Some(emitter) => emitter,
                    None => {
                        println!(
                            "No {:?} client connected.",
                            ws_message.target_api_specifier()
                        );
                        return Ok(());
                    }
                };

                for emitter in emitters {
                    emitter.send(ws_message.clone().into()).unwrap();
                }

                Ok(())
            }
        })
        .await
        .unwrap_or_default();
}
