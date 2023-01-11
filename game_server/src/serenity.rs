use std::sync::{Mutex, mpsc::{Sender, SendError}};

use serenity::prelude::*;

pub struct Dispatcher(Sender<String>);

impl Dispatcher {
    pub fn new(sender: Sender<String>) -> Dispatcher {
        Dispatcher(sender)
    }

    pub fn dispatch(&self, message: String) -> Result<(), SendError<String>> {
        self.0.send(message)
    }
}

impl TypeMapKey for Dispatcher {
    type Value = Mutex<Option<Dispatcher>>;
}