use std::sync::Arc;

use serde::{Deserialize, Serialize};
use ws_protocol::serenity::BjornMessageHandler;

use crate::MessageHandler;

#[derive(Serialize, Deserialize)]
pub enum Message {
    StartupBegin,
    StartupComplete,
    ShutdownBegin,
    ShutdownComplete,
    Info(String),
    Chat(String, String),
}

pub struct Api;

impl ws_protocol::ClientApi for Api {
    type Message = Message;

    fn id() -> &'static str {
        "minecraft_client"
    }
}

pub struct Handler {
    data: Arc<tokio::sync::RwLock<serenity::prelude::TypeMap>>,
    cache_and_http: Arc<serenity::CacheAndHttp>,
}

impl Handler {
    pub fn new(client: &serenity::Client) -> Handler {
        Handler {
            data: client.data.clone(),
            cache_and_http: client.cache_and_http.clone(),
        }
    }
}

impl ws_protocol::ClientApiHandler for Handler {
    type Api = Api;

    fn handle_message(&mut self, message: Message) {
        // TODO: I should think of a better way to do this.
        tokio::spawn(MessageHandler::server_message(
            self.data.clone(),
            self.cache_and_http.clone(),
            message,
        ));
    }
}
