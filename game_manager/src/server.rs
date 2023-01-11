mod minecraft;
use std::sync::mpsc;

use crate::ws::Client;

pub use self::minecraft::*;

pub trait GameServer {
    fn register_on_message_handler(&mut self, ws: &mut Client);
}

pub trait GameServerBuilder {
    type Configuration;

    fn name() -> &'static str;
    fn get_config() -> Option<Self::Configuration>;
    fn build(config: Self::Configuration) -> Box<dyn GameServer + Send + Sync>;
}

pub struct GameManager(Option<Client>);

impl GameManager {
    pub fn new() -> GameManager {
        GameManager(Some(Client::new()))
    }

    pub fn register<T>(&mut self)
    where
        T: GameServer + GameServerBuilder,
    {
        match T::get_config() {
            Some(config) => {
                T::build(config).register_on_message_handler(self.0.as_mut().unwrap());
                println!("{} server registered", T::name());
            }
            None => println!("{} not configured, skipping.", T::name()),
        }
    }

    pub fn wait_for_cancel(mut self) {
        let (sender, receiver) = mpsc::channel();
        let mut ws = self.0.take().unwrap();
        ctrlc::set_handler(move || {
            println!("^C");
            ws.shutdown();
            sender.send(()).unwrap_or_default();
        })
        .expect("ctrlc handler to work");

        receiver.recv().unwrap_or_default();
    }
}
