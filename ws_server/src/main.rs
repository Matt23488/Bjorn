use std::sync::{Arc, Mutex};

use ws_protocol::BjornWsServer;

fn main() {
    let server = Arc::new(Mutex::new(BjornWsServer::new()));

    // {
    //     let server = server.clone();
    //     ctrlc::set_handler(move || {
    //         println!("Ctrl+C detected");
    //         server
    //             .lock()
    //             .expect("lock to be valid")
    //             .stop();
    //     }).expect("Error setting Ctrl+C handler");
    // }

    server.lock().expect("lock to be valid").wait();
}
