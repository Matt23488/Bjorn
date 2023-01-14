#[cfg(feature = "tokio")]
pub mod server;

#[cfg(feature = "tokio")]
pub mod client;

use std::collections::VecDeque;

use tokio::sync::mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender};

type WsMessageContent = String;

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct WsMessage {
    id: Option<u64>,
    source: WsClientType,
    target: WsClientType,
    message: WsMessageContent,
}

#[derive(Debug, Hash, PartialEq, Eq, Copy, Clone, serde::Serialize, serde::Deserialize)]
pub enum WsClientType {
    Discord,
    Web,
    ServerManager,
}

const CLIENT_TYPE_DISCORD: &str = "discord";
const CLIENT_TYPE_WEB: &str = "web";
const CLIENT_TYPE_SERVER_MANAGER: &str = "server manager";

pub struct WsChannel {
    tx: Option<UnboundedSender<WsMessage>>,
    rx: Option<UnboundedReceiver<WsMessage>>,
    client_type: WsClientType,
    next_id: u64,
    response_queue: VecDeque<WsMessage>,
}

impl WsChannel {
    fn new(
        client_type: WsClientType,
        to_server: UnboundedSender<WsMessage>,
        from_server: UnboundedReceiver<WsMessage>,
    ) -> WsChannel {
        WsChannel {
            tx: Some(to_server),
            rx: Some(from_server),
            client_type,
            next_id: 0,
            response_queue: VecDeque::new(),
        }
    }

    pub fn pair(client_type: WsClientType) -> (WsChannel, WsChannel) {
        let (to_server, from_client) = unbounded_channel();
        let (to_client, from_server) = unbounded_channel();

        let client_channel = WsChannel::new(client_type, to_client, from_client);
        let server_channel = WsChannel::new(client_type, to_server, from_server);

        return (client_channel, server_channel);
    }

    pub async fn request(&mut self, recipient: WsClientType, message: String) -> String {
        let next_id = self.next_id;
        self.next_id += 1;

        let message = WsMessage {
            id: Some(next_id),
            source: self.client_type,
            target: recipient,
            message,
        };

        self.tx.as_ref().unwrap().send(message).unwrap();

        loop {
            match self.next_response().await {
                WsMessage {
                    id: Some(id),
                    message,
                    ..
                } => {
                    if id == next_id {
                        break message;
                    }
                }
                incoming => self.response_queue.push_back(incoming),
            }
        }
    }

    pub async fn on_request<F: futures_util::Future<Output = String>>(
        &mut self,
        handler: impl Fn(String) -> F + 'static,
    ) {
        let handler = Box::pin(handler);
        loop {
            let WsMessage {
                id,
                source,
                message,
                ..
            } = self.next_response().await;

            let response = WsMessage {
                id,
                target: source,
                source: self.client_type,
                message: handler(message).await,
            };

            self.tx.as_ref().unwrap().send(response).unwrap();
        }
    }

    pub fn split(mut self) -> (UnboundedSender<WsMessage>, UnboundedReceiver<WsMessage>) {
        (self.tx.take().unwrap(), self.rx.take().unwrap())
    }

    async fn next_response(&mut self) -> WsMessage {
        if self.response_queue.len() > 0 {
            return self.response_queue.pop_front().unwrap();
        }

        self.rx.as_mut().unwrap().recv().await.unwrap()
    }
}

impl TryFrom<tungstenite::Message> for WsMessage {
    type Error = String;

    fn try_from(message: tungstenite::Message) -> Result<Self, Self::Error> {
        match message {
            tungstenite::Message::Text(json) => match serde_json::from_str(&json) {
                Ok(msg) => Ok(msg),
                Err(e) => Err(format!("Error deserializing message: {e}")),
            },
            _ => Err(format!("Unexpected message: {message:?}")),
        }
    }
}
