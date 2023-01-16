use game_server::ServerProcess;

use crate::server_process::MinecraftServerProcess;

pub struct MinecraftProcConfig {
    server_path: String,
}

impl MinecraftProcConfig {
    fn with_server_path(server_path: String) -> MinecraftProcConfig {
        MinecraftProcConfig { server_path }
    }
}

pub struct MinecraftServer {
    process: MinecraftServerProcess,
}

impl game_server::SupportedGame for MinecraftServer {
    fn id() -> &'static str {
        "minecraft"
    }

    fn display_name() -> &'static str {
        "Minecraft"
    }
}

impl game_server::GameServerBuilder for MinecraftServer {
    type Config = MinecraftProcConfig;

    fn get_config() -> Option<Self::Config> {
        std::env::var("BJORN_MINECRAFT_SERVER")
            .map(MinecraftProcConfig::with_server_path)
            .ok()
    }

    fn build(config: Self::Config) -> Box<dyn game_server::GameServer + Send + Sync> {
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
    };
}

impl game_server::GameServer for MinecraftServer {
    fn handle_message(&mut self, message: &str) -> String {
        let command = message
            .chars()
            .take_while(|c| !c.is_whitespace())
            .collect::<String>();
        let rest = message.chars().skip(command.len()).collect::<String>();
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
