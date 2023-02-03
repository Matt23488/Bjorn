use discord_config::BjornMessageHandler;
use std::sync::Arc;

use serde::{Deserialize, Serialize};

use crate::MessageHandler;

#[derive(Debug, Serialize, Deserialize)]
pub enum Message {
    StartupBegin,
    StartupComplete(String),
    ShutdownBegin,
    ShutdownComplete,
    Info(String),
    PlayerJoined(String),
    PlayerQuit(String),
    PlayerDied(String),
    Haldor(Vec<(f32, f32)>),
    MobAttack(String),
}

impl Message {
    pub fn indicates_follow_up(&self) -> bool {
        match self {
            Message::StartupBegin | Message::ShutdownBegin => true,
            _ => false,
        }
    }
}

pub struct Api;

impl ws_protocol::ClientApi for Api {
    type Message = Message;

    fn id() -> &'static str {
        "valheim_client"
    }
}

pub struct Handler {
    data: Arc<tokio::sync::RwLock<serenity::prelude::TypeMap>>,
    cache_and_http: Arc<serenity::CacheAndHttp>,
}

impl Handler {
    pub fn new(
        data: Arc<tokio::sync::RwLock<serenity::prelude::TypeMap>>,
        cache_and_http: Arc<serenity::CacheAndHttp>,
    ) -> Handler {
        Handler {
            data,
            cache_and_http,
        }
    }
}

impl ws_protocol::ClientApiHandler for Handler {
    type Api = Api;

    fn handle_message(&mut self, message: <Self::Api as ws_protocol::ClientApi>::Message) {
        tokio::spawn(MessageHandler::server_message(
            self.data.clone(),
            self.cache_and_http.clone(),
            message,
        ));
    }
}
