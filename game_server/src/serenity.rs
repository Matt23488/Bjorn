use std::sync::{
    mpsc::{SendError, Sender},
    Mutex,
};

use serenity::prelude::*;

pub struct Dispatcher(Sender<String>);

impl Dispatcher {
    pub fn new(sender: Sender<String>) -> Dispatcher {
        Dispatcher(sender)
    }

    pub fn dispatch(&self, message: String) -> DispatchResult {
        self.0.send(message)
    }
}

impl TypeMapKey for Dispatcher {
    type Value = Mutex<Option<Dispatcher>>;
}

pub type DispatchResult = Result<(), SendError<String>>;
