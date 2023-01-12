use std::sync::mpsc;

pub struct GameManager(Option<game_server::MessageReceiver>);

impl GameManager {
    pub fn new() -> GameManager {
        GameManager(Some(game_server::MessageReceiver::new()))
    }

    pub fn register<T>(&mut self)
    where
        T: game_server::GameServer + game_server::GameServerBuilder,
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
