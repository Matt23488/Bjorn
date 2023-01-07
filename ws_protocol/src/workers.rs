use std::{
    collections::HashMap,
    net::TcpStream,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
    thread::{self, JoinHandle},
    time::Duration,
};

use websocket::{
    server::NoTlsAcceptor,
    sync::{Client, Server, Writer},
    ClientBuilder, CloseData, Message, OwnedMessage, WebSocketError,
};

use crate::BjornWsClientType;

pub fn spawn_client_worker(
    client_type: BjornWsClientType,
    cancellation_token: Arc<AtomicBool>,
    ws_client: Arc<Mutex<Option<Writer<TcpStream>>>>,
) -> JoinHandle<()> {
    thread::spawn(move || {
        while !cancellation_token.load(Ordering::SeqCst) {
            println!("connecting to server");
            let mut client = match ClientBuilder::new("ws://127.0.0.1:42069")
                .unwrap()
                .connect_insecure()
            {
                Ok(client) => client,
                Err(_) => {
                    println!("Failed to connect to ws_server. Retrying in 5 seconds...");
                    thread::sleep(Duration::from_secs(5));
                    continue;
                }
            };

            match client.recv_message() {
                Ok(OwnedMessage::Text(welcome)) => match welcome.as_str() {
                    "Bjorn" => (),
                    _ => {
                        kill_client(client, "Unrecognized welcome message");
                        return;
                    }
                },
                _ => println!("error"),
            };

            let (mut receiver, sender) = client.split().unwrap();

            *ws_client.lock().unwrap() = Some(sender);

            ws_client
                .lock()
                .unwrap()
                .as_mut()
                .unwrap()
                .send_message(&Message::from(client_type))
                .unwrap();
            let cancellation_token = Arc::new(AtomicBool::new(false));
            let cancelled = cancellation_token.clone();

            let ws_mutex = ws_client.clone();
            let ping_loop = thread::spawn(move || {
                while !cancelled.load(Ordering::SeqCst) {
                    ws_mutex
                        .lock()
                        .unwrap()
                        .as_mut()
                        .unwrap()
                        .send_message(&OwnedMessage::Ping(vec![4, 20, 69]))
                        .unwrap();
                    thread::sleep(Duration::from_secs(5));
                }
            });

            let mut close_reason = None;
            for message in receiver.incoming_messages() {
                match message {
                    Ok(OwnedMessage::Close(data)) => {
                        close_reason = data.map(|CloseData { reason, .. }| reason);
                        break;
                    }
                    Ok(message) => println!("Received: {message:?}"),
                    Err(WebSocketError::NoDataAvailable) => (),
                    Err(err) => {
                        close_reason = Some(err.to_string());
                        break;
                    }
                }
            }

            println!(
                "Connection closed: {}",
                close_reason.unwrap_or(String::from("No reason specified."))
            );

            receiver.shutdown().unwrap();
            cancellation_token.store(true, Ordering::SeqCst);
            ping_loop.join().unwrap();
        }
    })
}

type BjornWsServer = Server<NoTlsAcceptor>;

pub fn spawn_server_worker(server: BjornWsServer) -> JoinHandle<()> {
    let client_map = Arc::new(Mutex::new(HashMap::new()));

    thread::spawn(move || {
        for connection in server.filter_map(Result::ok) {
            let client_map = client_map.clone();
            thread::spawn(move || {
                let mut client = connection.accept().unwrap();

                let message = Message::text("Bjorn");
                let _ = client.send_message(&message);

                let message = match client.recv_message() {
                    Ok(message) => message,
                    Err(_) => {
                        kill_client(client, "Error getting message.");
                        return;
                    }
                };

                let client_type = BjornWsClientType::from(&message);

                if let client_type @ BjornWsClientType::Invalid = client_type {
                    kill_client(client, client_type.as_str());
                    return;
                }

                let (mut receiver, sender) = client.split().unwrap();
                let sender = Arc::new(Mutex::new(sender));

                client_map
                    .lock()
                    .unwrap()
                    .insert(client_type, sender.clone());

                let mut close_reason = None;
                for message in receiver.incoming_messages() {
                    match message {
                        Ok(OwnedMessage::Close(data)) => {
                            close_reason = data.map(|CloseData { reason, .. }| reason);
                            break;
                        }
                        Ok(OwnedMessage::Ping(data)) => sender
                            .lock()
                            .unwrap()
                            .send_message(&Message::pong(data))
                            .unwrap(),
                        Ok(message) => println!("Received: {message:?}"),
                        Err(WebSocketError::NoDataAvailable) => (),
                        Err(e) => println!("{e}"),
                    }
                }

                let sender = client_map.lock().unwrap().remove(&client_type).unwrap();
                println!(
                    "Connection closed: {}",
                    close_reason.unwrap_or(String::from("No reason specified."))
                );
                sender.lock().unwrap().shutdown().unwrap();
            });
        }
    })
}

fn kill_client(mut client: Client<TcpStream>, reason: &str) {
    let _ = client.send_message(&Message::close_because(u16::MAX, reason));
    client.shutdown().unwrap();
}
