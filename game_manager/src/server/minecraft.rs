use game_server::ServerProcess;
use minecraft::MinecraftServerProcess;

use super::{GameServer, GameServerBuilder};
use crate::ws::OnMessage;

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

impl GameServerBuilder for MinecraftServer {
    type Configuration = MinecraftConfig;

    fn name() -> &'static str {
        "Minecraft"
    }

    fn get_config() -> Option<Self::Configuration> {
        std::env::var("BJORN_MINECRAFT_SERVER")
            .map(MinecraftConfig::with_server_path)
            .ok()
    }

    fn build(config: Self::Configuration) -> Box<dyn GameServer + Send + Sync> {
        Box::new(MinecraftServer {
            process: Some(
                MinecraftServerProcess::build(config.server_path)
                    .expect("Minecraft environment to be configured properly"),
            ),
        })
    }
}

impl GameServer for MinecraftServer {
    fn register_on_message_handler(&mut self, ws: &mut crate::ws::Client) {
        let mut minecraft = self.process.take().unwrap();
        ws.on_message(move |message: minecraft::Message| match message {
            minecraft::Message::Start => {
                if let Err(e) = minecraft.start() {
                    eprintln!("Error starting server: {e}");
                }
            }
            minecraft::Message::Stop => {
                if let Err(e) = minecraft.stop() {
                    eprintln!("Error stopping server: {e}");
                }
            }
            minecraft::Message::Save => {
                if let Err(e) = minecraft.save() {
                    eprintln!("Error saving world: {e}");
                }
            }
            minecraft::Message::Say(message) => {
                if let Err(e) = minecraft.say(message) {
                    eprintln!("Error sending message to server: {e}");
                }
            }
            minecraft::Message::Tp(args) => {
                if let Err(e) = minecraft.tp(args) {
                    eprintln!("Error teleporting player: {e}");
                }
            }
            minecraft::Message::Unknown => (),
        });
    }
}
