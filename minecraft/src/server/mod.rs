mod process;
use std::sync::{Arc, Mutex};

use process::*;

use serde::{Deserialize, Serialize};

use crate::client;

#[derive(Debug, Serialize, Deserialize)]
pub enum Message {
    Start,
    Stop,
    Save,
    Chat(String, String),
    Tp(String, String),
    QueryPlayers,
}

pub struct Api;

impl ws_protocol::ClientApi for Api {
    type Message = Message;

    fn id() -> &'static str {
        "minecraft_server"
    }
}

macro_rules! captures {
    ($re:expr, $line:expr) => {
        $re.captures($line)
            .map(|captures| {
                captures
                    .iter()
                    .skip(1)
                    .flat_map(|c| c)
                    .map(|c| c.as_str())
                    .collect::<Vec<_>>()
            })
            .as_ref()
            .map(|c| c.as_slice())
    };
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

        let mut server_process = MinecraftServerProcess::build(server_dir.as_str());
        let players = Arc::new(Mutex::new(vec![]));

        let client_api = Arc::new(Mutex::new(client_api));
        let chat_regex = regex::Regex::new(r"<(\S+)>\s(.*)|\*\s(\S+)\s(.*)$").unwrap();
        let startup_finished_regex =
            regex::Regex::new(r#"\[Server thread/INFO\]: Done \(.+\)! For help, type "help"$"#)
                .unwrap();

        let player_joined_regex = regex::Regex::new(r"(\S+) joined the game$").unwrap();
        let player_quit_regex = regex::Regex::new(r"(\S+) left the game$").unwrap();

        let advancement_regex =
            regex::Regex::new(r"(\S+) has made the advancement [(.+)]$").unwrap();

        let death_regex =
            regex::Regex::new(r"\[Server thread/INFO\]: ([a-zA-Z0-9_]+) (.+)$").unwrap();

        {
            let client_api = client_api.clone();
            let players = players.clone();
            server_process.handle_stdout(move |line| {
                let chat_captures = chat_regex.captures(line).map(|captures| {
                    captures
                        .iter()
                        .flat_map(|c| c)
                        .map(|c| c.as_str())
                        .collect::<Vec<_>>()
                });

                if let Some([_line, player, message]) = chat_captures.as_ref().map(|c| c.as_slice())
                {
                    client_api.lock().unwrap().send(client::Message::Chat(
                        String::from(*player),
                        String::from(*message),
                    ));
                    return;
                }

                if let Some([player]) = captures!(player_joined_regex, line) {
                    players.lock().unwrap().push(String::from(*player));
                    client_api
                        .lock()
                        .unwrap()
                        .send(client::Message::PlayerJoined(String::from(*player)));
                    return;
                }

                if let Some([player]) = captures!(player_quit_regex, line) {
                    let mut players = players.lock().unwrap();
                    let player_index = players.iter().position(|p| p == player).unwrap();
                    players.remove(player_index);

                    client_api
                        .lock()
                        .unwrap()
                        .send(client::Message::PlayerQuit(String::from(*player)));
                    return;
                }

                if let Some([player, advancement]) = captures!(advancement_regex, line) {
                    client_api
                        .lock()
                        .unwrap()
                        .send(client::Message::PlayerAdvancement(
                            String::from(*player),
                            String::from(*advancement),
                        ));

                    return;
                }

                if let Some([player, death_message]) = captures!(death_regex, line) {
                    if !death_message.starts_with("lost connection")
                        && players
                            .lock()
                            .unwrap()
                            .iter()
                            .find(|p| p == player)
                            .is_some()
                    {
                        client_api.lock().unwrap().send(client::Message::PlayerDied(
                            String::from(*player),
                            String::from(*death_message),
                        ));

                        return;
                    }
                }

                if startup_finished_regex.is_match(line) {
                    client_api
                        .lock()
                        .unwrap()
                        .send(client::Message::StartupComplete);
                    return;
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
}

impl ws_protocol::ClientApiHandler for Handler {
    type Api = Api;

    fn handle_message(&mut self, message: Message) {
        let client_api = self.client_api.lock().unwrap();
        match message {
            Message::Start => match self.server_process.start() {
                Ok(_) => Ok(client_api.send(client::Message::StartupBegin)),
                Err(e) => Err(e),
            },
            Message::Stop => {
                client_api.send(client::Message::ShutdownBegin);
                match self.server_process.stop() {
                    Ok(_) => Ok(client_api.send(client::Message::ShutdownComplete)),
                    Err(e) => Err(e),
                }
            }
            Message::Save => match self.server_process.save() {
                Ok(_) => Ok(client_api.send(client::Message::Info("World saved.".into()))),
                Err(e) => Err(e),
            },
            Message::Chat(user, message) => {
                match self.server_process.chat(user.as_str(), message.as_str()) {
                    Ok(_) => Ok(()),
                    Err(_) => Ok(()), // Don't care to report errors on chat messages
                }
            }
            Message::Tp(player, target) => {
                match self.server_process.tp(player.as_str(), target.as_str()) {
                    Ok(_) => Ok(()),
                    Err(e) => Err(e),
                }
            }
            Message::QueryPlayers => {
                client_api.send(client::Message::Players(
                    self.players.lock().unwrap().clone(),
                ));
                Ok(())
            }
        }
        .unwrap_or_else(|e| client_api.send(client::Message::Info(e.to_string())));
    }
}
