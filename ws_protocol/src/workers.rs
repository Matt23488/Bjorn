use std::{
    collections::HashMap,
    net::TcpStream,
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc,
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

use crate::{BjornWsClientType, MessageCallback};

pub fn spawn_client_worker(
    client_type: BjornWsClientType,
    cancellation_token: Arc<AtomicBool>,
    ws_client: Arc<Mutex<Option<Writer<TcpStream>>>>,
    message_sender: mpsc::Sender<String>,
    message_receiver: mpsc::Receiver<String>,
) -> JoinHandle<()> {
    let message_receiver = Arc::new(Mutex::new(message_receiver));
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

            let ws_message_loop = {
                let ws_client = ws_client.clone();
                let receiver = message_receiver.clone();
                thread::spawn(move || loop {
                    let message = receiver.lock().unwrap().recv();

                    match message {
                        Ok(message) => match message.as_str() {
                            "!@#$QUIT$#@!" => break,
                            message => {
                                if let Some(ws_client) = &mut *ws_client.lock().unwrap() {
                                    ws_client.send_message(&Message::text(message)).unwrap();
                                }
                            }
                        },
                        Err(_) => {
                            break;
                        }
                    }
                })
            };

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

            ws_client.lock().unwrap().take();
            receiver.shutdown().unwrap();
            cancellation_token.store(true, Ordering::SeqCst);
            message_sender.send("!@#$QUIT$#@!".into()).unwrap();
            ws_message_loop.join().unwrap();
        }
    })
}

type BjornWsServer = Server<NoTlsAcceptor>;

pub fn spawn_server_worker(server: BjornWsServer, message_callback: MessageCallback, cancellation_token: Arc<AtomicBool>) -> JoinHandle<()> {
    let client_map = Arc::new(Mutex::new(HashMap::new()));

    thread::spawn(move || {
        for connection in server.filter_map(Result::ok).take_while(|_| !cancellation_token.load(Ordering::SeqCst)) {
            let client_map = client_map.clone();
            let message_callback = message_callback.clone();
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

                println!("{client_type:?} client connected");

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
                        Ok(OwnedMessage::Text(text)) => message_callback.lock().unwrap()(text),
                        Ok(message) => println!("Received from {client_type:?}: {message:?}"),
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
