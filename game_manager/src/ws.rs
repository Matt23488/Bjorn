use std::sync::{Arc, Mutex};

use ws_protocol::{BjornWsClient, BjornWsClientType};

pub struct Client {
    ws: Option<BjornWsClient>,
    callbacks: Arc<Mutex<Vec<(&'static str, Box<dyn FnMut(String) + Send + 'static>)>>>,
}

impl Client {
    pub fn new() -> Client {
        let ws = BjornWsClient::new(BjornWsClientType::ServerManager);
        let callbacks = Arc::new(Mutex::new(Vec::new()));

        let mut client = Client {
            ws: Some(ws),
            callbacks: callbacks.clone(),
        };

        client.ws.as_mut().unwrap().on_message(move |text| {
            let parts = text.split_whitespace().collect::<Vec<_>>();
            let target = parts[0];
            let rest = parts.into_iter().skip(1).collect::<Vec<_>>().join(" ");

            if let Some((_, cb)) = callbacks.lock().unwrap().iter_mut().find(|(game, _)| *game == target) {
                cb(rest);
            }
        });

        client
    }

    pub fn shutdown(mut self) {
        self.ws.take().unwrap().shutdown();
    }
}

pub trait OnMessage<MessageType> {

    fn on_message<F>(&mut self, callback: F)
    where
        F: FnMut(MessageType) + Send + 'static;
}

impl OnMessage<minecraft::Message> for Client {
    fn on_message<F>(&mut self, mut callback: F)
        where
            F: FnMut(minecraft::Message) + Send + 'static,
    {
        self.callbacks.lock().unwrap().push(("minecraft", Box::new(move |message| {
            let parts = message.split_whitespace().collect::<Vec<_>>();
            let cmd = parts[0];
            let args = parts.into_iter().skip(1).collect::<Vec<_>>().join(" ");
            
            let message = match cmd {
                "start" => minecraft::Message::Start,
                "stop" => minecraft::Message::Stop,
                "save" => minecraft::Message::Save,
                "say" => minecraft::Message::Say(args),
                "tp" => minecraft::Message::Tp(args),
                _ => minecraft::Message::Unknown,
            };

            callback(message);
        })));
    }
}