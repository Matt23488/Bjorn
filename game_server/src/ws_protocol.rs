use std::sync::{Arc, Mutex};

pub struct MessageReceiver {
    ws: Option<ws_protocol::WsClient>,
    pub callbacks: Arc<Mutex<Vec<(&'static str, Box<dyn FnMut(String) + Send + 'static>)>>>,
}

impl MessageReceiver {
    pub fn new() -> MessageReceiver {
        let ws = ws_protocol::WsClient::new(ws_protocol::WsClientType::ServerManager);
        let callbacks = Arc::new(Mutex::new(Vec::new()));

        let mut client = MessageReceiver {
            ws: Some(ws),
            callbacks: callbacks.clone(),
        };

        client.ws.as_mut().unwrap().on_message(move |text| {
            let parts = text.split_whitespace().collect::<Vec<_>>();
            let target = parts[0];
            let rest = parts.into_iter().skip(1).collect::<Vec<_>>().join(" ");

            if let Some((_, cb)) = callbacks
                .lock()
                .unwrap()
                .iter_mut()
                .find(|(game, _)| *game == target)
            {
                cb(rest);
            }
        });

        client
    }

    pub fn shutdown(&mut self) {
        self.ws.take().unwrap().shutdown();
    }
}

pub trait OnMessage<MessageType> {
    fn on_message<F>(&mut self, callback: F)
    where
        F: FnMut(MessageType) + Send + 'static;
}

pub trait GameServer {
    fn register_on_message_handler(&mut self, ws: &mut MessageReceiver);
}

pub trait GameServerBuilder {
    type Configuration;

    fn name() -> &'static str;
    fn get_config() -> Option<Self::Configuration>;
    fn build(config: Self::Configuration) -> Box<dyn GameServer + Send + Sync>;
}
