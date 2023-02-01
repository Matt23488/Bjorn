use ws_protocol::WsTask;

pub async fn run() {
    let addr = std::env::var("BJORN_WS_CONNECT_ADDRESS").unwrap();

    let (minecraft_ws_api, minecraft_api_runner, minecraft_api_canceller) = ws_protocol::WsClient::<minecraft::client::Api>::new();
    let (minecraft_ws_handler, minecraft_handler_canceller) = ws_protocol::WsClientHandler::new(minecraft::server::Handler::new(minecraft_ws_api));
    let (valheim_ws_api, valheim_api_runner, valheim_api_canceller) = ws_protocol::WsClient::<valheim::client::Api>::new();
    let (valheim_ws_handler, valheim_handler_canceller) = ws_protocol::WsClientHandler::new(valheim::server::Handler::new(valheim_ws_api));

    let mut minecraft_api_canceller = Some(minecraft_api_canceller);
    let mut minecraft_handler_canceller = Some(minecraft_handler_canceller);
    let mut valheim_api_canceller = Some(valheim_api_canceller);
    let mut valheim_handler_canceller = Some(valheim_handler_canceller);
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
        _ = tokio::spawn(valheim_ws_handler.run(addr.clone())) => {}
        _ = tokio::spawn(valheim_api_runner.run(addr.clone())) => {}
        _ = tokio::spawn(minecraft_ws_handler.run(addr.clone())) => {}
        _ = minecraft_api_runner.run(addr) => {}
    }
}
