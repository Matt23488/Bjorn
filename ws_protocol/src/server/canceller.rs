use std::sync::{Arc, Mutex};

use tokio::sync::{mpsc, oneshot};
use tungstenite::protocol::{frame::coding::CloseCode, CloseFrame};

pub struct Canceller {
    cancel: oneshot::Sender<()>,
    senders: Arc<Mutex<Vec<mpsc::UnboundedSender<tungstenite::Message>>>>,
}

impl Canceller {
    pub fn new(
        cancel: oneshot::Sender<()>,
        mut sender_rx: mpsc::UnboundedReceiver<mpsc::UnboundedSender<tungstenite::Message>>,
    ) -> Canceller {
        let senders = Arc::new(Mutex::new(vec![]));

        {
            let senders = senders.clone();
            tokio::spawn(async move {
                loop {
                    let sender = match sender_rx.recv().await {
                        Some(sender) => sender,
                        None => break,
                    };

                    senders.lock().unwrap().push(sender);
                }
            });
        }

        Canceller { cancel, senders }
    }

    pub fn cancel(self) {
        for sender in self.senders.lock().unwrap().iter() {
            sender
                .send(tungstenite::Message::Close(Some(CloseFrame {
                    code: CloseCode::Error,
                    reason: "WS Server received SIGINT".into(),
                })))
                .unwrap();
        }

        self.cancel.send(()).unwrap();
    }
}
