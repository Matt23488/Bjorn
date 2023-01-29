use std::sync::Arc;

use discord_config::BjornMessageHandler;
use serde::{Deserialize, Serialize};
use serenity::model::prelude::{Mention, UserId};

use crate::{MessageHandler, Players};

#[derive(Serialize, Deserialize)]
pub enum Message {
    StartupBegin,
    StartupComplete,
    ShutdownBegin,
    ShutdownComplete,
    Info(String),
    Chat(String, String),
    PlayerJoined(String),
    PlayerQuit(String),
    PlayerDied(String, String),
    PlayerAdvancement(String, String),
    Players(Vec<String>),
}

macro_rules! with_mention {
    ($players:expr, $player:expr) => {{
        $players
            .get_user_id(&$player)
            .map(|id| Mention::User(UserId(id)).to_string())
            .unwrap_or($player)
    }};
}

impl Message {
    pub fn to_string(self, players: &Players) -> String {
        match self {
            Message::StartupBegin => "Minecraft Server starting...".into(),
            Message::StartupComplete => "Minecraft Server startup complete.".into(),
            Message::ShutdownBegin => "Minecraft Server shutting down...".into(),
            Message::ShutdownComplete => "Minecraft Server shutdown complete.".into(),
            Message::Info(message) => format!("[Info] {message}"),
            Message::Chat(player, message) => {
                format!("[In-Game] {}: {}", with_mention!(players, player), message)
            }
            Message::PlayerJoined(player) => {
                format!("{} joined the server!", with_mention!(players, player))
            }
            Message::PlayerQuit(player) => {
                format!("{} left the server.", with_mention!(players, player))
            }
            Message::PlayerDied(player, message) => {
                format!("{} {}.", with_mention!(players, player), message)
            }
            Message::PlayerAdvancement(player, message) => format!(
                "{} has made the advancement: `{}`!",
                with_mention!(players, player),
                message
            ),
            Message::Players(player_list) => {
                if player_list.len() > 0 {
                    player_list
                        .into_iter()
                        .map(|p| with_mention!(players, p))
                        .collect::<Vec<_>>()
                        .join(", ")
                } else {
                    "There are currently no players on the server.".into()
                }
            }
        }
    }
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
