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
    PlayerAdvancement(String, String, String),
    Players(Vec<String>),
    Command(String, String, String),
    NamedEntityDied(String, String),
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
            Message::PlayerAdvancement(player, text, advancement) => format!(
                "{} has {} `{}`!",
                with_mention!(players, player),
                text,
                advancement,
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
            Message::Command(player, command, target) => format!(
                "[In-Game] {}: !{} {}",
                with_mention!(players, player),
                command,
                target
            ),
            Message::NamedEntityDied(entity, message) => format!(
                "It is with great sadness I bring news that our beloved `{entity}` {message}."
            ),
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
        "minecraft_client"
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

    fn handle_message(&mut self, message: Message) {
        // TODO: I should think of a better way to do this.
        tokio::spawn(MessageHandler::server_message(
            self.data.clone(),
            self.cache_and_http.clone(),
            message,
        ));
    }
}
