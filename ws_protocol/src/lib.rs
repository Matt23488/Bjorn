use std::collections::HashMap;
use std::net::TcpStream;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};

// TODO: Look into tokio-tungstenite to replace websocket
// TODO: https://github.com/snapview/tokio-tungstenite/blob/master/examples/server.rs
use websocket::sync::{Server, Writer};
use websocket::Message;
use websocket::{CloseData, OwnedMessage};
use workers::{spawn_client_worker, spawn_server_worker};

mod workers;

#[derive(Debug, Hash, PartialEq, Eq, Copy, Clone)]
pub enum BjornWsClientType {
    Invalid,
    Discord,
    Web,
    // TODO: Minecraft, Valheim
}

const CLIENT_TYPE_INVALID: &str = "Invalid client type";
const CLIENT_TYPE_DISCORD: &str = "discord";
const CLIENT_TYPE_WEB: &str = "web";

impl BjornWsClientType {
    fn as_str(self) -> &'static str {
        match self {
            BjornWsClientType::Invalid => CLIENT_TYPE_INVALID,
            BjornWsClientType::Discord => CLIENT_TYPE_DISCORD,
            BjornWsClientType::Web => CLIENT_TYPE_WEB,
        }
    }
}

impl From<&BjornWsClientType> for OwnedMessage {
    fn from(client: &BjornWsClientType) -> Self {
        match client {
            BjornWsClientType::Invalid => OwnedMessage::Close(Some(CloseData {
                status_code: 1,
                reason: String::from(CLIENT_TYPE_INVALID),
            })),
            BjornWsClientType::Discord => OwnedMessage::Text(String::from(CLIENT_TYPE_DISCORD)),
            BjornWsClientType::Web => OwnedMessage::Text(String::from(CLIENT_TYPE_WEB)),
        }
    }
}

impl From<BjornWsClientType> for Message<'_> {
    fn from(client: BjornWsClientType) -> Self {
        match client {
            BjornWsClientType::Invalid => Message::close_because(1, CLIENT_TYPE_INVALID),
            BjornWsClientType::Discord => Message::text(CLIENT_TYPE_DISCORD),
            BjornWsClientType::Web => Message::text(CLIENT_TYPE_WEB),
        }
    }
}

impl From<&OwnedMessage> for BjornWsClientType {
    fn from(message: &OwnedMessage) -> Self {
        match message {
            OwnedMessage::Text(text) => match text.as_str() {
                CLIENT_TYPE_DISCORD => BjornWsClientType::Discord,
                CLIENT_TYPE_WEB => BjornWsClientType::Web,
                _ => BjornWsClientType::Invalid,
            },
            _ => BjornWsClientType::Invalid,
        }
    }
}

pub struct BjornWsClient {
    thread: Option<thread::JoinHandle<()>>,
    cancellation_token: Arc<AtomicBool>,
    ws_client: Arc<Mutex<Option<Writer<TcpStream>>>>,
    sender: Arc<Mutex<Option<Sender<String>>>>,
}

impl BjornWsClient {
    pub fn new(client_type: BjornWsClientType) -> Self {
        let cancellation_token = Arc::new(AtomicBool::new(false));
        let ws_client = Arc::new(Mutex::new(None));
        let (sender, receiver) = std::sync::mpsc::channel::<String>();

        let thread = spawn_client_worker(
            client_type,
            cancellation_token.clone(),
            ws_client.clone(),
            sender.clone(),
            receiver,
        );

        BjornWsClient {
            thread: Some(thread),
            cancellation_token,
            ws_client,
            sender: Arc::new(Mutex::new(Some(sender))),
        }
    }

    pub fn send_message<S: Into<String>>(&self, message: S) -> Result<(), String> {
        match (
            self.ws_client.lock().unwrap().as_ref(),
            self.sender.lock().unwrap().as_ref(),
        ) {
            (Some(_), Some(sender)) => match sender.send(message.into()) {
                Ok(_) => Ok(()),
                Err(err) => Err(err.to_string()),
            },
            _ => Err("Sender closed".into()),
        }
    }
}

#[cfg(feature = "serenity")]
use serenity::prelude::*;

#[cfg(feature = "serenity")]
impl TypeMapKey for BjornWsClient {
    type Value = HashMap<String, Arc<Mutex<Option<BjornWsClient>>>>;
}

struct ShutdownMessage(&'static str);

impl From<ShutdownMessage> for OwnedMessage {
    fn from(message: ShutdownMessage) -> Self {
        OwnedMessage::Close(Some(CloseData {
            status_code: u16::MAX,
            reason: String::from(message.0),
        }))
    }
}

impl BjornWsClient {
    pub fn shutdown(mut self) {
        if let Some(thread) = self.thread.take() {
            drop(self.sender.lock().unwrap().take());
            self.cancellation_token.store(true, Ordering::SeqCst);
            if let Some(ws_client) = self.ws_client.lock().unwrap().as_mut() {
                ws_client
                    .send_message(&OwnedMessage::from(ShutdownMessage("shutdown() called")))
                    .unwrap();
            }
            thread.join().unwrap();
        }
    }
}

pub struct BjornWsServer {
    connection_thread: Option<JoinHandle<()>>,
}

impl BjornWsServer {
    pub fn new() -> Self {
        let server = Server::bind("0.0.0.0:42069").unwrap();

        let connection_thread = spawn_server_worker(server);

        BjornWsServer {
            connection_thread: Some(connection_thread),
        }
    }
}

impl BjornWsServer {
    pub fn join_connection_thread(mut self) {
        if let Some(thread) = self.connection_thread.take() {
            println!("Joining connection thread...");
            thread.join().unwrap();
        }
    }
}
