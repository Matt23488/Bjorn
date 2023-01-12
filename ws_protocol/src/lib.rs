// use std::collections::HashMap;
use std::net::TcpStream;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Sender};
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
pub enum WsClientType {
    Invalid,
    Discord,
    Web,
    ServerManager,
}

const CLIENT_TYPE_INVALID: &str = "Invalid client type";
const CLIENT_TYPE_DISCORD: &str = "discord";
const CLIENT_TYPE_WEB: &str = "web";
const CLIENT_TYPE_SERVER_MANAGER: &str = "server manager";

impl WsClientType {
    fn as_str(self) -> &'static str {
        match self {
            WsClientType::Invalid => CLIENT_TYPE_INVALID,
            WsClientType::Discord => CLIENT_TYPE_DISCORD,
            WsClientType::Web => CLIENT_TYPE_WEB,
            WsClientType::ServerManager => CLIENT_TYPE_SERVER_MANAGER,
        }
    }
}

impl From<WsClientType> for Message<'_> {
    fn from(client: WsClientType) -> Self {
        match client {
            WsClientType::Invalid => Message::close_because(1, CLIENT_TYPE_INVALID),
            WsClientType::Discord => Message::text(CLIENT_TYPE_DISCORD),
            WsClientType::Web => Message::text(CLIENT_TYPE_WEB),
            WsClientType::ServerManager => Message::text(CLIENT_TYPE_SERVER_MANAGER),
        }
    }
}

impl From<&OwnedMessage> for WsClientType {
    fn from(message: &OwnedMessage) -> Self {
        match message {
            OwnedMessage::Text(text) => match text.as_str() {
                CLIENT_TYPE_DISCORD => WsClientType::Discord,
                CLIENT_TYPE_WEB => WsClientType::Web,
                CLIENT_TYPE_SERVER_MANAGER => WsClientType::ServerManager,
                _ => WsClientType::Invalid,
            },
            _ => WsClientType::Invalid,
        }
    }
}

type Callback<T> = dyn FnMut(T) + Send + 'static;
pub struct OptionCallback<T>(Mutex<Option<Box<Callback<T>>>>);

impl<T> OptionCallback<T> {
    fn none() -> OptionCallback<T> {
        OptionCallback(Mutex::new(None))
    }

    fn _some<F>(callback: F) -> OptionCallback<T>
    where
        F: FnMut(T) + Send + 'static,
    {
        OptionCallback(Mutex::new(Some(Box::new(callback))))
    }

    fn replace<F>(&self, callback: F)
    where
        F: FnMut(T) + Send + 'static,
    {
        self.0.lock().unwrap().replace(Box::new(callback));
    }

    fn execute(&self, arg: T) {
        if let Some(callback) = self.0.lock().unwrap().as_mut() {
            callback(arg);
        }
    }
}

pub struct WsClient {
    thread: Option<thread::JoinHandle<()>>,
    cancellation_token: Arc<AtomicBool>,
    ws_client: Arc<Mutex<Option<Writer<TcpStream>>>>,
    sender: Arc<Mutex<Option<Sender<String>>>>,
    message_callback: Arc<OptionCallback<String>>,
}

impl WsClient {
    pub fn new(client_type: WsClientType) -> Self {
        let cancellation_token = Arc::new(AtomicBool::new(false));
        let ws_client = Arc::new(Mutex::new(None));
        let (sender, receiver) = mpsc::channel::<String>();

        let message_callback = Arc::new(OptionCallback::none());

        let thread = spawn_client_worker(
            client_type,
            cancellation_token.clone(),
            ws_client.clone(),
            sender.clone(),
            receiver,
            message_callback.clone(),
        );

        WsClient {
            thread: Some(thread),
            cancellation_token,
            ws_client,
            sender: Arc::new(Mutex::new(Some(sender))),
            message_callback,
        }
    }

    pub fn on_message<F>(&mut self, callback: F)
    where
        F: FnMut(String) + Send + 'static,
    {
        self.message_callback.replace(callback);
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

    pub fn shutdown(mut self) {
        if let Some(thread) = self.thread.take() {
            self.cancellation_token.store(true, Ordering::SeqCst);
            if let Some(ws_client) = self.ws_client.lock().unwrap().as_mut() {
                ws_client
                    .send_message(&OwnedMessage::from(ShutdownMessage("shutdown() called")))
                    .unwrap();
            }
            thread.join().unwrap();
        }
    }

    pub fn wait(&mut self) {
        if let Some(thread) = self.thread.take() {
            thread.join().unwrap();
        }
    }
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

pub struct WsServer {
    connection_thread: Option<JoinHandle<()>>,
    cancellation_token: Arc<AtomicBool>,
}

impl WsServer {
    pub fn new() -> Self {
        let server = Server::bind("0.0.0.0:42069").unwrap();

        let cancellation_token = Arc::new(AtomicBool::new(false));
        let connection_thread = spawn_server_worker(server, cancellation_token.clone());

        WsServer {
            connection_thread: Some(connection_thread),
            cancellation_token,
        }
    }

    pub fn wait(&mut self) {
        if let Some(thread) = self.connection_thread.take() {
            thread.join().unwrap();
        }
    }

    pub fn stop(&self) {
        self.cancellation_token.store(true, Ordering::SeqCst);
    }
}
