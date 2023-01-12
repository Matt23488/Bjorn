use game_server::{OnMessage, ServerProcess};

use crate::server::MinecraftServerProcess;

pub enum Message {
    Unknown,
    Start,
    Stop,
    Save,
    Say(String),
    Tp(String),
}

pub struct MinecraftConfig {
    server_path: String,
}

impl MinecraftConfig {
    fn with_server_path(server_path: String) -> MinecraftConfig {
        MinecraftConfig { server_path }
    }
}

pub struct MinecraftServer {
    process: Option<MinecraftServerProcess>,
}

impl game_server::GameServerBuilder for MinecraftServer {
    type Configuration = MinecraftConfig;

    fn name() -> &'static str {
        "Minecraft"
    }

    fn get_config() -> Option<Self::Configuration> {
        std::env::var("BJORN_MINECRAFT_SERVER")
            .map(MinecraftConfig::with_server_path)
            .ok()
    }

    fn build(config: Self::Configuration) -> Box<dyn game_server::GameServer + Send + Sync> {
        Box::new(MinecraftServer {
            process: Some(
                MinecraftServerProcess::build(config.server_path)
                    .expect("Minecraft environment to be configured properly"),
            ),
        })
    }
}

impl game_server::GameServer for MinecraftServer {
    fn register_on_message_handler(&mut self, ws: &mut game_server::MessageReceiver) {
        let mut minecraft = self.process.take().unwrap();
        ws.on_message(move |message: Message| match message {
            Message::Start => {
                if let Err(e) = minecraft.start() {
                    eprintln!("Error starting server: {e}");
                }
            }
            Message::Stop => {
                if let Err(e) = minecraft.stop() {
                    eprintln!("Error stopping server: {e}");
                }
            }
            Message::Save => {
                if let Err(e) = minecraft.save() {
                    eprintln!("Error saving world: {e}");
                }
            }
            Message::Say(message) => {
                if let Err(e) = minecraft.say(message) {
                    eprintln!("Error sending message to server: {e}");
                }
            }
            Message::Tp(args) => {
                if let Err(e) = minecraft.tp(args) {
                    eprintln!("Error teleporting player: {e}");
                }
            }
            Message::Unknown => (),
        });
    }
}

impl game_server::OnMessage<Message> for game_server::MessageReceiver {
    fn on_message<F>(&mut self, mut callback: F)
    where
        F: FnMut(Message) + Send + 'static,
    {
        self.callbacks.lock().unwrap().push((
            "minecraft",
            Box::new(move |message| {
                let parts = message.split_whitespace().collect::<Vec<_>>();
                let cmd = parts[0];
                let args = parts.into_iter().skip(1).collect::<Vec<_>>().join(" ");

                let message = match cmd {
                    "start" => Message::Start,
                    "stop" => Message::Stop,
                    "save" => Message::Save,
                    "say" => Message::Say(args),
                    "tp" => Message::Tp(args),
                    _ => Message::Unknown,
                };

                callback(message);
            }),
        ));
    }
}
