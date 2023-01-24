use ws_protocol::WsTask;

pub async fn run() {
    let addr = std::env::var("BJORN_WS_CONNECT_ADDRESS").unwrap();

    let (ws_api, api_runner, api_canceller) =
        ws_protocol::WsClient::<minecraft::client::Api>::new();
    let (ws_handler, handler_canceller) =
        ws_protocol::WsClientHandler::new(minecraft::server::Handler::new(ws_api));

    let mut api_canceller = Some(api_canceller);
    let mut handler_canceller = Some(handler_canceller);
    ctrlc::set_handler(move || {
        println!("^C");

        if let Some(canceller) = api_canceller.take() {
            canceller.cancel();
        }

        if let Some(canceller) = handler_canceller.take() {
            canceller.cancel();
        }
    })
    .expect("Ctrl+C Handler failed");

    tokio::select! {
        _ = tokio::spawn(ws_handler.run(addr.clone())) => {}
        _ = api_runner.run(addr) => {}
    }
}
