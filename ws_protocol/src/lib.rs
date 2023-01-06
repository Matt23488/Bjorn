// use std::borrow::BorrowMut;
use std::net::TcpStream;
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use websocket::{ClientBuilder, OwnedMessage, WebSocketError};
use websocket::sync::{Server, Client};
use websocket::Message;

// pub enum BjornWsClientType {
//     Discord,

// }

struct CancellationToken(bool);

impl CancellationToken {
    fn cancel(&mut self) {
        self.0 = true;
    }
}

pub struct BjornWsClient {
    thread: Option<thread::JoinHandle<()>>,
    cancellation_token: Arc<Mutex<CancellationToken>>,
}

impl BjornWsClient {
    pub fn new(_client_name: &str) -> Self {
        let cancellation_token = Arc::new(Mutex::new(CancellationToken(false)));

        let cancellation_token_ = cancellation_token.clone();
        let thread = thread::spawn(move || {
            loop {
                if cancellation_token_.lock().unwrap().0 {
                    break;
                }

                println!("connecting to server");
                let mut client = match ClientBuilder::new("ws://127.0.0.1:42069")
                    .unwrap()
                    .connect_insecure() {
                        Ok(client) => client,
                        Err(_) => {
                            println!("Failed to connect to ws_server. Retrying in 5 seconds...");
                            thread::sleep(Duration::from_secs(5));
                            continue;
                        },
                    };
        
                // let (mut receiver, mut _sender) = client.split().unwrap();
                let _handshake = match client.recv_message() {
                    Ok(OwnedMessage::Text(welcome)) => println!("{welcome}"),
                    _ => println!("error")
                };

        
                for message in client.incoming_messages() {
                    match message {
                        Ok(OwnedMessage::Close(_)) => break,
                        Ok(message) => println!("Received: {message:?}"),
                        Err(WebSocketError::NoDataAvailable) => (),
                        Err(e) => println!("{e}"),
                    }
                }

                println!("Connection closed");
                client.shutdown().unwrap();
            }

            // receiver.shutdown().unwrap();
            // _sender.shutdown().unwrap();
        });

        BjornWsClient {
            thread: Some(thread),
            cancellation_token,
        }
    }
}

impl Drop for BjornWsClient {
    fn drop(&mut self) {
        if let Some(thread) = self.thread.take() {
            self.cancellation_token.lock().unwrap().cancel();
            thread.join().unwrap();
        }
    }
}

pub struct BjornWsServer {
    connection_thread: Option<JoinHandle<()>>,
    clients: Arc<Mutex<Vec<Client<TcpStream>>>>,
}

impl BjornWsServer {
    pub fn new() -> Self {
        let server = Server::bind("127.0.0.1:42069").unwrap();

        let clients = Arc::new(Mutex::new(Vec::new()));
        // let conn_clients = clients.clone();

        let connection_thread = thread::spawn(move || {
            for connection in server.filter_map(Result::ok) {
                // let conn_clients = conn_clients.clone();
                thread::spawn(move || {
                    let mut client = connection.accept().unwrap();

                    let message = Message::text("Bjorn");
                    let _ = client.send_message(&message);

                    // conn_clients.lock().unwrap().push(client);

                    // * Just a test that closes the connection after five seconds
                    thread::sleep(Duration::from_secs(5));

                    // client.shutdown().unwrap();
                    let _ = client.send_message(&Message::close());
                    client.shutdown().unwrap();

                    // TODO: Listen for messages and shut down client. Also figure out which client is connected.
                });
            }
        });

        BjornWsServer {
            connection_thread: Some(connection_thread),
            clients,
        }
    }
}

impl Drop for BjornWsServer {
    fn drop(&mut self) {
        if let Some(thread) = self.connection_thread.take() {
            println!("Joining connection thread...");
            thread.join().unwrap();
        }

        self.clients
            .lock()
            .unwrap()
            .iter()
            .for_each(|client| {
                println!("Joining client thread...");
                client.shutdown().unwrap();
            });
    }
}