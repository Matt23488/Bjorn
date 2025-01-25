mod parser;
use parser::*;

mod process;
use process::*;

mod utils;
use utils::*;

use std::sync::{Arc, Mutex};

use serde::{Deserialize, Serialize};

use crate::client;

#[derive(Serialize, Deserialize)]
pub enum Message {
    Start(bool),
    Stop,
    QueryHaldor,
}

pub struct Api;

impl ws_protocol::ClientApi for Api {
    type Message = Message;

    fn id() -> &'static str {
        "valheim_server"
    }
}

pub struct ConnectedPlayer {
    id: String,
    name: String,
}

pub struct Handler {
    client_api: Arc<Mutex<ws_protocol::WsClient<client::Api>>>,
    server_process: ValheimServerProcess,
    _players: Arc<Mutex<Vec<ConnectedPlayer>>>,
    world_path: String,
}

impl Handler {
    pub fn new(client_api: ws_protocol::WsClient<client::Api>) -> Handler {
        let server_dir = std::env::var("BJORN_VALHEIM_SERVER")
            .expect("Valheim server directory not configured.");
        let server_name =
            std::env::var("BJORN_VALHEIM_NAME").expect("Valheim server name not configured.");
        let world_name =
            std::env::var("BJORN_VALHEIM_WORLD").expect("Valheim world name not configured.");
        let password =
            std::env::var("BJORN_VALHEIM_PASSWORD").expect("Valheim password not configured.");
        let world_path =
            std::env::var("BJORN_VALHEIM_WORLD_DB").expect("Valheim world path not configured.");
        let app_id =
            std::fs::read_to_string(std::path::Path::new(&server_dir).join("steam_appid.txt"))
                .expect("Error reading Valheim Steam AppID");

        let mut server_process =
            ValheimServerProcess::build(&server_dir, &server_name, &world_name, &password, &app_id);
        let players = Arc::new(Mutex::new(vec![]));

        let client_api = Arc::new(Mutex::new(client_api));

        {
            let client_api = client_api.clone();
            let players = players.clone();
            let running = Arc::new(Mutex::new(false));

            {
                let running = running.clone();
                server_process.on_stopped(move || {
                    *running.lock().unwrap() = false;
                })
            }

            server_process.handle_stdout(move |line, crossplay| {
                let state = ServerState {
                    players: players.clone(),
                    running: running.clone(),
                    crossplay,
                };

                if let Some(message) = parse_line(line, state) {
                    if let (true, client::Message::StartupComplete(Some(_)))
                    | (false, client::Message::StartupComplete(None)) = (crossplay, &message)
                    {
                        *running.lock().unwrap() = true;
                    }

                    client_api.lock().unwrap().send(message);
                }

                println!("[Valheim] {line}");
            });
        }

        Handler {
            client_api,
            server_process,
            _players: players,
            world_path,
        }
    }

    pub fn is_configured() -> bool {
        std::env::var("BJORN_VALHEIM_SERVER").is_ok()
    }
}

impl ws_protocol::ClientApiHandler for Handler {
    type Api = Api;

    fn handle_message(&mut self, message: <Self::Api as ws_protocol::ClientApi>::Message) {
        let client_api = self.client_api.lock().unwrap();
        match message {
            Message::Start(crossplay) => self
                .server_process
                .start(crossplay)
                .map(|_| client_api.send(client::Message::StartupBegin)),
            Message::Stop => match self.server_process.is_running() {
                true => self
                    .server_process
                    .stop()
                    .map(|_| client_api.send(client::Message::ShutdownComplete)),
                false => Ok(client_api.send(client::Message::Info(String::from(
                    "Server is already stopped.",
                )))),
            },
            Message::QueryHaldor => Ok(client_api.send(client::Message::Haldor(
                get_haldor_locations(&self.world_path),
            ))),
        }
        .unwrap_or_else(|e| client_api.send(client::Message::Info(e.to_string())));
    }
}
