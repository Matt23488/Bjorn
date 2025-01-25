use ws_protocol::WsTask;

pub async fn run() {
    let addr = std::env::var("BJORN_WS_CONNECT_ADDRESS").unwrap();

    let (minecraft_api_runner, minecraft_ws_handler, mut minecraft_api_canceller, mut minecraft_handler_canceller) = match minecraft::server::Handler::is_configured() {
        true => {
            let (api, runner, canceller) =
                ws_protocol::WsClient::<minecraft::client::Api>::new();
            let (handler, handler_canceller) =
                ws_protocol::WsClientHandler::new(minecraft::server::Handler::new(api));

            (
                runner.run(addr.clone()),
                handler.run(addr.clone()),
                Some(canceller),
                Some(handler_canceller),
            )
        }
        false => (DummyStruct::run("minecraft_api_runner"), DummyStruct::run("minecraft_ws_handler"), None, None),
    };

    let (valheim_api_runner, valheim_ws_handler, mut valheim_api_canceller, mut valheim_handler_canceller) = match valheim::server::Handler::is_configured() {
        true => {
            let (api, runner, canceller) =
                ws_protocol::WsClient::<valheim::client::Api>::new();
            let (handler, handler_canceller) =
                ws_protocol::WsClientHandler::new(valheim::server::Handler::new(api));

            (
                runner.run(addr.clone()),
                handler.run(addr.clone()),
                Some(canceller),
                Some(handler_canceller),
            )
        }
        false => (DummyStruct::run("valheim_api_runner"), DummyStruct::run("valheim_ws_handler"), None, None),
    };

    ctrlc::set_handler(move || {
        println!("^C");

        if let Some(canceller) = minecraft_api_canceller.take() {
            canceller.cancel();
        }

        if let Some(canceller) = minecraft_handler_canceller.take() {
            canceller.cancel();
        }

        if let Some(canceller) = valheim_api_canceller.take() {
            canceller.cancel();
        }

        if let Some(canceller) = valheim_handler_canceller.take() {
            canceller.cancel();
        }
    })
    .expect("Ctrl+C Handler failed");

    tokio::select! {
        _ = valheim_ws_handler => {}
        _ = valheim_api_runner => {}
        _ = minecraft_ws_handler => {}
        _ = minecraft_api_runner => {}
    }
}

use async_trait::async_trait;

#[async_trait]
trait DummyTask {
    async fn run(task_name: &'static str);
}

struct DummyStruct;

#[async_trait]
impl DummyTask for DummyStruct {
    async fn run(task_name: &'static str) {
        println!("{task_name}: Dummy task, sleeping for 5 seconds");
        tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
    }
}
