use tokio::sync::mpsc::{self, UnboundedReceiver, UnboundedSender};

use crate::message::Message;

pub fn dual_channel() -> (Endpoint, Endpoint) {
    let (emitter_message_sink, handler_message_stream) = mpsc::unbounded_channel();
    let (handler_message_sink, emitter_message_stream) = mpsc::unbounded_channel();

    (
        Endpoint::new(emitter_message_sink, emitter_message_stream),
        Endpoint::new(handler_message_sink, handler_message_stream),
    )
}

pub struct Endpoint {
    message_sink: UnboundedSender<Message>,
    message_stream: UnboundedReceiver<Message>,
}

impl Endpoint {
    pub fn new(
        message_sink: UnboundedSender<Message>,
        message_stream: UnboundedReceiver<Message>,
    ) -> Self {
        Endpoint {
            message_sink,
            message_stream,
        }
    }

    pub fn send(&self, message: Message) {
        self.message_sink.send(message).unwrap();
    }

    pub async fn handle<F>(&mut self, mut f: F)
    where
        F: FnMut(Message),
    {
        loop {
            let message = match self.message_stream.recv().await {
                Some(msg) => msg,
                None => break,
            };

            f(message);
        }
    }

    pub fn split(self) -> (UnboundedSender<Message>, UnboundedReceiver<Message>) {
        let Endpoint {
            message_sink,
            message_stream,
        } = self;
        (message_sink, message_stream)
    }
}
