use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread::{self, JoinHandle};

use websocket::{OwnedMessage, CloseData};
use websocket::sync::Server;
use websocket::Message;
use workers::{spawn_client_worker, spawn_server_worker};

mod workers;

#[derive(Hash, PartialEq, Eq, Copy, Clone)]
pub enum BjornWsClientType {
    Invalid,
    Discord,
    WebServer,
    // TODO: Minecraft, Valheim
}

impl BjornWsClientType {
    fn as_str(self) -> &'static str {
        match self {
            BjornWsClientType::Invalid => "Invalid client type",
            BjornWsClientType::Discord => "discord",
            BjornWsClientType::WebServer => "web_server",
        }
    }
}

impl From<&BjornWsClientType> for OwnedMessage {
    fn from(client: &BjornWsClientType) -> Self {
        match client {
            BjornWsClientType::Invalid => OwnedMessage::Close(Some(CloseData { status_code: 1, reason: String::from("Invalid client type") })),
            BjornWsClientType::Discord => OwnedMessage::Text(String::from("discord")),
            BjornWsClientType::WebServer => OwnedMessage::Text(String::from("web_server")),
        }
    }
}

impl From<BjornWsClientType> for Message<'_> {
    fn from(client: BjornWsClientType) -> Self {
        match client {
            BjornWsClientType::Invalid => Message::close_because(1, "Invalid client type"),
            BjornWsClientType::Discord => Message::text("discord"),
            BjornWsClientType::WebServer => Message::text("web_server"),
        }
    }
}

impl From<&OwnedMessage> for BjornWsClientType {
    fn from(message: &OwnedMessage) -> Self {
        match message {
            OwnedMessage::Text(text) => match text.as_str() {
                "discord" => BjornWsClientType::Discord,
                "web_server" => BjornWsClientType::WebServer,
                _ => BjornWsClientType::Invalid,
            }
            _ => BjornWsClientType::Invalid,
        }
    }
}

pub struct BjornWsClient {
    thread: Option<thread::JoinHandle<()>>,
    cancellation_token: Arc<AtomicBool>,
}

impl BjornWsClient {
    pub fn new(client_type: BjornWsClientType) -> Self {
        let cancellation_token = Arc::new(AtomicBool::new(false));

        let thread = spawn_client_worker(client_type, cancellation_token.clone());

        BjornWsClient {
            thread: Some(thread),
            cancellation_token,
        }
    }
}

impl Drop for BjornWsClient {
    fn drop(&mut self) {
        if let Some(thread) = self.thread.take() {
            self.cancellation_token.store(true, Ordering::SeqCst);
            thread.join().unwrap();
        }
    }
}

pub struct BjornWsServer {
    connection_thread: Option<JoinHandle<()>>,
}

impl BjornWsServer {
    pub fn new() -> Self {
        let server = Server::bind("127.0.0.1:42069").unwrap();

        let connection_thread = spawn_server_worker(server);

        BjornWsServer {
            connection_thread: Some(connection_thread),
        }
    }
}

impl Drop for BjornWsServer {
    fn drop(&mut self) {
        if let Some(thread) = self.connection_thread.take() {
            println!("Joining connection thread...");
            thread.join().unwrap();
        }
    }
}