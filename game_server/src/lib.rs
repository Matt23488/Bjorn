pub trait ServerProcess {
    fn build(dir: &str) -> Result<Self, String>
    where
        Self: Sized;
    fn start(&mut self) -> Result<(), String>;
    fn stop(&mut self) -> Result<(), String>;
}

pub trait SupportedGame {
    fn id() -> &'static str;
    fn display_name() -> &'static str;
}

pub trait GameServer {
    fn handle_message(&mut self, message: &str) -> String;
}

pub trait GameServerBuilder {
    type Config;

    fn get_config() -> Option<Self::Config>;
    fn build(config: Self::Config) -> Box<dyn GameServer + Send + Sync>;
}
