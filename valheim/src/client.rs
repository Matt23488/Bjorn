use std::sync::Arc;
use discord_config::BjornMessageHandler;
use serenity::model::prelude::*;

use serde::{Serialize, Deserialize};

use crate::{Players, MessageHandler};

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
}

macro_rules! with_mention {
    ($players:expr, $player:expr) => {{
        $players
            .get_user_id($player)
            .map(|id| Mention::User(UserId(id)).to_string())
            .unwrap_or($player.clone())
    }};
}

impl Message {
    pub fn to_string(&self, players: &Players) -> String {
        match self {
            Message::StartupBegin => "Valheim Server starting...".into(),
            Message::StartupComplete(code) => format!("Valheim Server startup complete. Join Code is `{code}`."),
            Message::ShutdownBegin => "Valheim Server shutting down...".into(),
            Message::ShutdownComplete => "Valheim Server shutdown complete.".into(),
            Message::Info(message) => format!("[Info] {message}"),
            Message::PlayerJoined(player) => {
                format!("{} joined the server!", with_mention!(players, player))
            }
            Message::PlayerQuit(player) => {
                format!("{} left the server.", with_mention!(players, player))
            }
            Message::PlayerDied(player) => {
                format!("{} died.", with_mention!(players, player))
            }
        }
    }

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
