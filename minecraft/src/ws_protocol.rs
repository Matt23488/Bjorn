use game_server::ServerProcess;

use crate::server_process::MinecraftServerProcess;

pub enum Message {
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
    process: MinecraftServerProcess,
}

impl game_server::GameServerBuilder for MinecraftServer {
    type Configuration = MinecraftConfig;

    fn id() -> &'static str {
        "minecraft"
    }

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
            process: MinecraftServerProcess::build(config.server_path.as_str())
                .expect("Minecraft environment to be configured properly"),
        })
    }
}

macro_rules! run_command {
    ($e:expr, $ok:literal, $err:literal) => {
        match $e {
            Ok(()) => $ok.into(),
            Err(e) => {
                eprintln!("{}: {}", $err, e);
                format!("{}: {}", $err, e)
            }
        }
        .into()
    };
}

impl game_server::GameServer for MinecraftServer {
    fn handle_message(&mut self, message: &str) -> String {
        let command = message
            .chars()
            .take_while(|c| !c.is_whitespace())
            .collect::<String>();
        let rest = message.chars().skip(command.len()).collect::<String>(); //.trim();
        let rest = rest.trim();

        match command.as_str() {
            "start" => run_command!(
                self.process.start(),
                "Server starting",
                "Error starting server"
            ),
            "stop" => run_command!(
                self.process.stop(),
                "Server stopping",
                "Error stopping server"
            ),
            "save" => run_command!(self.process.save(), "Server saving", "Error saving world"),
            "say" => run_command!(self.process.say(rest), "Chat sent", "Error sending chat"),
            "tp" => run_command!(
                self.process.tp(rest),
                "Player teleported",
                "Error teleporting player"
            ),
            _ => "Unrecognized command".into(),
        }
    }
}
