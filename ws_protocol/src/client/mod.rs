mod mpsc;
use mpsc::*;

use std::marker::PhantomData;

use async_trait::async_trait;

mod ws_task;
pub use ws_task::*;

mod runner;
pub use runner::*;

mod canceller;
pub use canceller::*;

use tokio::sync::oneshot;

use crate::{message::Message, ApiSpecifier};

pub trait ClientApi: Send + Sync {
    type Message: Send + Sync + serde::Serialize + for<'de> serde::Deserialize<'de>;

    fn id() -> &'static str;
}

pub trait ClientApiHandler: Send + Sync {
    type Api: ClientApi;

    fn handle_message(&mut self, message: <Self::Api as ClientApi>::Message);
}

pub type WsClientComponents<Api> = (WsClient<Api>, Runner, Canceller);

pub struct WsClient<Api>
where
    Api: ClientApi,
{
    _api: PhantomData<Api>,
    emitter_endpoint: Endpoint,
}

impl<Api> WsClient<Api>
where
    Api: ClientApi,
{
    pub fn new() -> WsClientComponents<Api> {
        let (emitter_endpoint, handler_endpoint) = mpsc::dual_channel();
        let (cancel, on_cancel) = oneshot::channel();

        (
            WsClient {
                _api: PhantomData,
                emitter_endpoint,
            },
            Runner::new(
                ApiSpecifier::Emits(Api::id().into()),
                handler_endpoint,
                on_cancel,
            ),
            Canceller::new(cancel),
        )
    }

    pub fn send(&self, message: Api::Message) {
        self.emitter_endpoint.send(Message {
            target: Api::id().into(),
            content: serde_json::to_string(&message).unwrap(),
        });
    }
}

pub type WsClientHandlerComponents<Api, Handler> = (WsClientHandler<Api, Handler>, Canceller);

pub struct WsClientHandler<Api, Handler>
where
    Api: ClientApi,
    Handler: ClientApiHandler<Api = Api> + 'static,
{
    handler: Handler,
    handler_endpoint: Endpoint,
    runner: Runner,
}

impl<Api, Handler> WsClientHandler<Api, Handler>
where
    Api: ClientApi,
    Handler: ClientApiHandler<Api = Api> + 'static,
{
    pub fn new(handler: Handler) -> WsClientHandlerComponents<Api, Handler> {
        let (emitter_endpoint, handler_endpoint) = mpsc::dual_channel();
        let (cancel, on_cancel) = oneshot::channel();

        (
            WsClientHandler {
                handler,
                handler_endpoint,
                runner: Runner::new(
                    ApiSpecifier::Handles(Api::id().into()),
                    emitter_endpoint,
                    on_cancel,
                ),
            },
            Canceller::new(cancel),
        )
    }
}

#[async_trait]
impl<Api, Handler> WsTask for WsClientHandler<Api, Handler>
where
    Api: ClientApi,
    Handler: ClientApiHandler<Api = Api> + 'static,
{
    async fn run(self, addr: String) {
        let WsClientHandler {
            mut handler,
            mut handler_endpoint,
            runner,
        } = self;

        let message_task = async move {
            handler_endpoint
                .handle(|message| {
                    handler.handle_message(serde_json::from_str(&message.content).unwrap())
                })
                .await;
        };

        tokio::select! {
            _ = tokio::spawn(message_task) => {}
            _ = runner.run(addr) => {}
        }
    }
}
