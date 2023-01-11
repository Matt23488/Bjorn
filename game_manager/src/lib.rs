use std::sync::{
    Arc, Mutex, mpsc,
};

use game_server::Server;
use minecraft::MinecraftServer;

mod ws;
use ws::{Client, OnMessage};

pub fn run() {
    let minecraft = Arc::new(Mutex::new(
        MinecraftServer::build().expect("build to succeed"),
    ));

    // create ws client and listen for messages
    let mut client = Client::new();

    client.on_message(move |message: minecraft::Message| match message {
        minecraft::Message::Start => {
            if let Err(e) = minecraft.lock().unwrap().start() {
                eprintln!("Error starting server: {e}");
            }
        }
        minecraft::Message::Stop => {
            if let Err(e) = minecraft.lock().unwrap().stop() {
                eprintln!("Error stopping server: {e}");
            }
        }
        minecraft::Message::Save => {
            if let Err(e) = minecraft.lock().unwrap().save() {
                eprintln!("Error saving world: {e}");
            }
        }
        minecraft::Message::Say(message) => {
            if let Err(e) = minecraft.lock().unwrap().say(message) {
                eprintln!("Error sending message to server: {e}");
            }
        }
        minecraft::Message::Tp(args) => {
            if let Err(e) = minecraft.lock().unwrap().tp(args) {
                eprintln!("Error teleporting player: {e}");
            }
        }
        minecraft::Message::Unknown => (),
    });

    let mut client = Some(client);
    let (sender, receiver) = mpsc::channel();

    ctrlc::set_handler(move || {
        println!("^C");

        if let Some(client) = client.take() {
            client.shutdown();
        }

        sender.send(()).unwrap_or_default();
    })
    .expect("ctrlc handler to work");

    receiver.recv().unwrap_or_default();
}
