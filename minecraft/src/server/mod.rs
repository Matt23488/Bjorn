mod fs;

mod parser;
use parser::*;

mod process;
use process::*;

use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};

use crate::client;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RealmCoords {
    id: String,
    x: f64,
    y: f64,
    z: f64,
}

impl RealmCoords {
    pub fn new(realm: &str, x: f64, y: f64, z: f64) -> Self {
        Self {
            id: realm.into(),
            x,
            y,
            z,
        }
    }

    pub fn realm_string(&self) -> &String {
        &self.id
    }

    pub fn coords(&self) -> (f64, f64, f64) {
        (
            self.x,
            self.y,
            self.z
        )
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub enum Message {
    Start,
    Stop,
    Save,
    Chat(String, String),
    Tp(String, String),
    TpLoc(String, RealmCoords),
    QueryPlayers,
    BackupWorld,
    Command(String),
}

pub struct Api;

impl ws_protocol::ClientApi for Api {
    type Message = Message;

    fn id() -> &'static str {
        "minecraft_server"
    }
}

pub struct Handler {
    client_api: Arc<Mutex<ws_protocol::WsClient<client::Api>>>,
    server_process: MinecraftServerProcess,
    players: Arc<Mutex<Vec<String>>>,
}

impl Handler {
    pub fn new(client_api: ws_protocol::WsClient<client::Api>) -> Handler {
        let server_dir = std::env::var("BJORN_MINECRAFT_SERVER")
            .expect("Minecraft server environment not properly configured.");

        let server_jar = std::env::var("BJORN_MINECRAFT_SERVER_JAR")
            .unwrap_or("server.jar".into());

        let max_memory = std::env::var("BJORN_MINECRAFT_MAX_MEMORY")
            .unwrap_or("4G".into());

        let world_name = std::env::var("BJORN_MINECRAFT_WORLD_NAME").unwrap_or("world".into());
        let backup_path = std::env::var("BJORN_MINECRAFT_BACKUP_PATH").ok();

        let mut server_process = MinecraftServerProcess::build(&server_dir, &server_jar, &max_memory, world_name, backup_path);
        let players = Arc::new(Mutex::new(vec![]));

        let client_api = Arc::new(Mutex::new(client_api));

        {
            let client_api = client_api.clone();
            let players = players.clone();

            server_process.handle_stdout(move |line| {
                if let Some(message) = parse_line(line, &players) {
                    client_api.lock().unwrap().send(message);
                }

                println!("[Minecraft] {line}");
            });
        }

        Handler {
            client_api,
            server_process,
            players,
        }
    }

    pub fn is_configured() -> bool {
        std::env::var("BJORN_MINECRAFT_SERVER").is_ok()
    }
}

impl ws_protocol::ClientApiHandler for Handler {
    type Api = Api;

    fn handle_message(&mut self, message: Message) {
        let client_api = self.client_api.lock().unwrap();
        match message {
            Message::Start => self
                .server_process
                .start()
                .map(|_| client_api.send(client::Message::StartupBegin)),
            Message::Stop => match self.server_process.is_running() {
                true => {
                    client_api.send(client::Message::ShutdownBegin);
                    match self.server_process.stop() {
                        Ok(_) => Ok(client_api.send(client::Message::ShutdownComplete)),
                        Err(e) => Err(e),
                    }
                }
                false => {
                    Ok(client_api.send(client::Message::Info("Server is already stopped.".into())))
                }
            },
            Message::Save => self
                .server_process
                .save()
                .map(|_| client_api.send(client::Message::Info("World saved.".into()))),
            Message::Chat(user, message) => {
                self.server_process
                    .chat(user.as_str(), message.as_str())
                    .unwrap_or_default();
                Ok(())
            }
            Message::Tp(player, target) => self.server_process.tp(player.as_str(), target.as_str()),
            Message::TpLoc(player, coords) => {
                let (x, y, z) = coords.coords();
                self.server_process
                    .tp_loc(&player, &coords.realm_string(), x, y, z)
            }
            Message::QueryPlayers => {
                client_api.send(client::Message::Players(
                    self.players.lock().unwrap().clone(),
                ));
                Ok(())
            }
            Message::BackupWorld => {
                client_api.send(client::Message::BackupBegin);

                self.server_process.backup_world()
                    .map(|WorldBackupResult{ dir_name, size }| client_api.send(client::Message::BackupComplete(dir_name, size)))
            }
            Message::Command(text) => {
                self.server_process.command(&format!("{text}\n"))
            }
        }
        .unwrap_or_else(|e| client_api.send(client::Message::Info(e.to_string())));
    }
}
