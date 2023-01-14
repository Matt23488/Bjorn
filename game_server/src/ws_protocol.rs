pub trait GameServer {
    fn handle_message(&mut self, message: &str) -> String;
}

pub trait GameServerBuilder {
    type Configuration;

    fn id() -> &'static str;
    fn name() -> &'static str;
    fn get_config() -> Option<Self::Configuration>;
    fn build(config: Self::Configuration) -> Box<dyn GameServer + Send + Sync>;
}
