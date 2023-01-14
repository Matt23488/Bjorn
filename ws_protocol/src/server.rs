use std::{
    collections::HashMap,
    error::Error,
    net::SocketAddr,
    sync::{Arc, Mutex},
};

use futures_channel::mpsc::{unbounded, UnboundedSender};
use futures_util::{future, pin_mut, stream::TryStreamExt, SinkExt, StreamExt};

use tokio::net::{TcpListener, TcpStream};

use crate::{
    WsClientType, WsMessage, CLIENT_TYPE_DISCORD, CLIENT_TYPE_SERVER_MANAGER, CLIENT_TYPE_WEB,
};

type Tx = UnboundedSender<tungstenite::Message>;

// TODO: This will potentially break with Web clients, whenever I open that up.
// TODO: Incoming connections of the same `WsClientType` will overwrite each other in the PeerMap.
type PeerMap = Arc<Mutex<HashMap<WsClientType, Tx>>>;

pub struct TaskRunner(Option<tokio::sync::oneshot::Receiver<()>>);
pub struct TaskCanceller(Option<tokio::sync::oneshot::Sender<()>>);

pub fn new() -> (TaskRunner, TaskCanceller) {
    let (sender, receiver) = tokio::sync::oneshot::channel();

    (TaskRunner(Some(receiver)), TaskCanceller(Some(sender)))
}

impl TaskRunner {
    pub async fn run(mut self, addr: String) {
        let ws_server_task = tokio::spawn(run(addr));
        let cancel_task = tokio::spawn(self.0.take().unwrap());

        tokio::select! {
            _ = ws_server_task => {}
            _ = cancel_task => {}
        };
    }
}

impl TaskCanceller {
    pub fn cancel(mut self) {
        self.0.take().unwrap().send(()).unwrap();
    }
}

#[derive(Debug)]
enum WsServerError {
    IoError(String),
}

impl std::fmt::Display for WsServerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                WsServerError::IoError(text) => format!("IO Error: {text}"),
            }
        )
    }
}

impl Error for WsServerError {}

impl From<std::io::Error> for WsServerError {
    fn from(e: std::io::Error) -> Self {
        WsServerError::IoError(e.to_string())
    }
}

async fn run(addr: String) -> Result<(), WsServerError> {
    let state = PeerMap::new(Mutex::new(HashMap::new()));

    let listener = TcpListener::bind(&addr).await?;
    println!("Listening on: {addr}");

    while let Ok((stream, addr)) = listener.accept().await {
        tokio::task::spawn(handle_connection(state.clone(), stream, addr));
    }

    Ok(())
}

async fn handle_connection(peer_map: PeerMap, raw_stream: TcpStream, addr: SocketAddr) {
    println!("Incoming TCP connection from: {addr}");

    let ws_stream = tokio_tungstenite::accept_async(raw_stream)
        .await
        .expect("Error during the websocket handshake occurred");

    let (mut outgoing, mut incoming) = ws_stream.split();

    println!("Attempting to send handshake token...");
    if let Err(_) = outgoing
        .send(tungstenite::Message::Text("Bjorn".into()))
        .await
    {
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

    let client_type = match WsClientType::try_from(handshake_response) {
        Ok(client_type) => client_type,
        Err(e) => {
            println!("Invalid client type: {e}");
            return;
        }
    };

    println!("WebSocket connection established: {client_type:?} ({addr})");

    let (tx, rx) = unbounded();
    peer_map.lock().unwrap().insert(client_type, tx);

    let broadcast_incoming = incoming.try_for_each(|msg| {
        let ws_message = match WsMessage::try_from(msg.clone()) {
            Ok(msg) => msg,
            Err(_) => {
                eprintln!("Unknown message, ignoring: {msg:?}");
                return future::ok(());
            }
        };

        let peers = peer_map.lock().unwrap();
        match peers.get(&ws_message.target) {
            Some(recipient) => recipient.unbounded_send(msg.clone()).unwrap(),
            None => {
                let message = format!("{:?} client not connected.", ws_message.target);
                println!("{message}");

                // Send a reply if the sender is expecting one.
                if let Some(id) = ws_message.id {
                    let sender = peers.get(&client_type).unwrap();
                    sender
                        .unbounded_send(
                            WsMessage {
                                id: Some(id),
                                target: ws_message.source,
                                source: ws_message.target,
                                message,
                            }
                            .into(),
                        )
                        .unwrap();
                }
            }
        }

        future::ok(())
    });

    let receive_from_others = rx.map(Ok).forward(outgoing);

    pin_mut!(broadcast_incoming, receive_from_others);
    future::select(broadcast_incoming, receive_from_others).await;

    println!("{client_type:?} ({addr}) disconnected");
    peer_map.lock().unwrap().remove(&client_type);
}

impl From<WsMessage> for tungstenite::Message {
    fn from(message: WsMessage) -> Self {
        tungstenite::Message::Text(serde_json::to_string(&message).unwrap())
    }
}

impl TryFrom<tungstenite::Message> for WsClientType {
    type Error = String;

    fn try_from(message: tungstenite::Message) -> Result<Self, Self::Error> {
        match message {
            tungstenite::Message::Text(client_type_str) => match client_type_str.as_str() {
                CLIENT_TYPE_DISCORD => Ok(WsClientType::Discord),
                CLIENT_TYPE_SERVER_MANAGER => Ok(WsClientType::ServerManager),
                CLIENT_TYPE_WEB => Ok(WsClientType::Web),
                val => Err(format!("Unknown client type: {val}")),
            },
            _ => Err(format!("Unexpected message: {message:?}")),
        }
    }
}
