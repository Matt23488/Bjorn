mod runner;
use runner::*;

mod canceller;
use canceller::*;

use tokio::sync::{mpsc, oneshot};

pub struct WsServer;

impl WsServer {
    pub fn new() -> (Runner, Canceller) {
        let (cancel, on_cancel) = oneshot::channel();
        let (sender_tx, sender_rx) = mpsc::unbounded_channel();

        (
            Runner::new(on_cancel, sender_tx),
            Canceller::new(cancel, sender_rx),
        )
    }
}
