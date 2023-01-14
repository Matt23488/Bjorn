#[cfg(feature = "serenity")]
mod serenity;

#[cfg(feature = "serenity")]
pub use self::serenity::*;

use std::{
    error::Error,
    sync::{Arc, Mutex},
    time::Duration,
};

use futures_util::{stream::TryStreamExt, SinkExt, StreamExt};

use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};
use tokio_tungstenite::connect_async;

use crate::{
    WsChannel, WsClientType, WsMessage, CLIENT_TYPE_DISCORD, CLIENT_TYPE_SERVER_MANAGER,
    CLIENT_TYPE_WEB,
};

pub struct TaskRunner(
    WsClientType,
    Option<WsChannel>,
    Option<tokio::sync::oneshot::Receiver<()>>,
);
pub struct TaskCanceller(Option<tokio::sync::oneshot::Sender<()>>);

pub fn new(client_type: WsClientType) -> (WsChannel, TaskRunner, TaskCanceller) {
    let (sender, receiver) = tokio::sync::oneshot::channel();
    let (client_channel, server_channel) = WsChannel::pair(client_type);

    (
        client_channel,
        TaskRunner(client_type, Some(server_channel), Some(receiver)),
        TaskCanceller(Some(sender)),
    )
}

impl TaskRunner {
    pub async fn run(mut self, addr: String) {
        let (to_client, from_client) = self.1.take().unwrap().split();
        let cancel_task = tokio::spawn(self.2.take().unwrap());

        let ws_client_task = tokio::spawn(async move {
            let to_client = Arc::new(Mutex::new(Some(to_client)));
            let from_client = Arc::new(Mutex::new(Some(from_client)));
            loop {
                if let Err(e) = run(&addr, self.0, to_client.clone(), from_client.clone()).await {
                    println!("WS Connection failure: {e}");
                }
                println!("No connection. Trying again in 5 seconds...");
                tokio::time::sleep(Duration::from_secs(5)).await;
            }
        });

        tokio::select! {
            _ = ws_client_task => {}
            _ = cancel_task  => {}
        };
    }
}

impl TaskCanceller {
    pub fn cancel(mut self) {
        self.0.take().unwrap().send(()).unwrap();
    }
}

#[derive(Debug)]
enum WsClientError {
    TungsteniteError(String),
    FailedReadHandshakeToken,
    InvalidHandshakeToken(String),
    SendMessageError,
}

impl std::fmt::Display for WsClientError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                WsClientError::TungsteniteError(text) => format!("Tungstenite error: {text}."),
                WsClientError::FailedReadHandshakeToken => "Couldn't read handshake token.".into(),
                WsClientError::InvalidHandshakeToken(token) =>
                    format!("Received invalid token: {token}"),
                WsClientError::SendMessageError => "Couldn't read message.".into(),
            }
        )
    }
}

impl Error for WsClientError {}

impl std::convert::From<tungstenite::Error> for WsClientError {
    fn from(e: tungstenite::Error) -> Self {
        WsClientError::TungsteniteError(e.to_string())
    }
}

async fn run(
    addr: &String,
    client_type: WsClientType,
    to_client: Arc<Mutex<Option<UnboundedSender<WsMessage>>>>,
    from_client: Arc<Mutex<Option<UnboundedReceiver<WsMessage>>>>,
) -> Result<(), WsClientError> {
    let (ws_stream, _r) = connect_async(addr).await?;

    let (mut write, read) = ws_stream.split();

    let mut read = Box::pin(read);
    match read.next().await {
        Some(Ok(tungstenite::Message::Text(text))) => {
            println!("Got text message from server: {text}");
            match text.as_str() {
                "Bjorn" => (),
                token => return Err(WsClientError::InvalidHandshakeToken(token.into())),
            }
        }
        _ => return Err(WsClientError::FailedReadHandshakeToken),
    }

    println!("Bjorn server identified. Sending handshake response...");
    let handshake_response = client_type.into();

    write.send(handshake_response).await?;

    let send_task = {
        let to_client = to_client.clone();
        let from_client = from_client.clone();
        tokio::spawn(async move {
            loop {
                // TODO: This is not cancellation safe as there is await between
                let mut fc = from_client.lock().unwrap().take().unwrap();
                let ws_message = match fc.recv().await {
                    Some(msg) => msg,
                    None => {
                        eprintln!("{}", WsClientError::SendMessageError);
                        from_client.lock().unwrap().replace(fc);
                        to_client
                            .lock()
                            .unwrap()
                            .as_ref()
                            .unwrap()
                            .send(WsMessage {
                                id: None,
                                target: client_type,
                                source: client_type,
                                message: WsClientError::SendMessageError.to_string(),
                            })
                            .unwrap();
                        continue;
                    }
                };

                from_client.lock().unwrap().replace(fc);

                let WsMessage {
                    id, source, target, ..
                } = ws_message;

                let message = ws_message.into();

                if let (true, Err(e)) = (id.is_some(), write.send(message).await) {
                    eprintln!("{e}");
                    to_client
                        .lock()
                        .unwrap()
                        .as_ref()
                        .unwrap()
                        .send(WsMessage {
                            id,
                            source: target,
                            target: source,
                            message: e.to_string(),
                        })
                        .unwrap();
                    continue;
                };
            }
        })
    };

    let recv_task = tokio::spawn(async move {
        read.try_for_each(|message| async {
            let json = match message {
                tungstenite::Message::Text(json) => json,
                invalid => {
                    eprintln!("Invalid message from server: {invalid:?}");
                    return Ok(());
                }
            };

            match serde_json::from_str::<WsMessage>(&json) {
                Ok(message) => {
                    to_client
                        .lock()
                        .unwrap()
                        .as_ref()
                        .unwrap()
                        .send(message)
                        .unwrap();
                }
                Err(_) => {
                    eprintln!("Invalid message from server: {json}");
                }
            }

            Ok(())
        })
        .await
        .unwrap_or_default();
    });

    let (a, b) = tokio::join!(send_task, recv_task);

    a.unwrap();
    b.unwrap();

    Ok(())
}

impl From<WsClientType> for tungstenite::Message {
    fn from(client_type: WsClientType) -> Self {
        match client_type {
            WsClientType::Discord => tungstenite::Message::Text(String::from(CLIENT_TYPE_DISCORD)),
            WsClientType::ServerManager => {
                tungstenite::Message::Text(String::from(CLIENT_TYPE_SERVER_MANAGER))
            }
            WsClientType::Web => tungstenite::Message::Text(String::from(CLIENT_TYPE_WEB)),
        }
    }
}
