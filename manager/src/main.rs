use std::thread;
use websocket::sync::Server;
use websocket::Message;

fn main() {
    let server = Server::bind("127.0.0.1:42069").unwrap();

    for connection in server.filter_map(Result::ok) {
        thread::spawn(move || {
            let mut client = connection.accept().unwrap();

            let message = Message::text("Hello, client!");
            let _ = client.send_message(&message);
        });
    }
}
