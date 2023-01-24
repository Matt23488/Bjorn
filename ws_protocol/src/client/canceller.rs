use tokio::sync::oneshot;

pub struct Canceller(oneshot::Sender<()>);

impl Canceller {
    pub fn new(cancel: oneshot::Sender<()>) -> Canceller {
        Canceller(cancel)
    }

    pub fn cancel(self) {
        self.0.send(()).unwrap();
    }
}
