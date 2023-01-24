use std::{
    collections::HashMap,
    env,
    sync::{Arc, Mutex},
};

pub struct GameManager {
    addr: Option<String>,
    request_channel: Option<Arc<Mutex<ws_protocol::WsChannel>>>,
    message_source: Option<tokio::sync::mpsc::UnboundedReceiver<ws_protocol::WsMessage>>,
    runner: Option<ws_protocol::client::TaskRunner>,
    canceller: Option<ws_protocol::client::TaskCanceller>,
    servers: Option<HashMap<&'static str, Box<dyn game_server::GameServer + Send>>>,
}

impl GameManager {
    pub fn new() -> GameManager {
        let addr = env::var("BJORN_WS_CONNECT_ADDRESS").unwrap();
        let (request_channel, message_source, runner, canceller) =
            ws_protocol::client::new(ws_protocol::WsClientType::ServerManager);

        GameManager {
            addr: Some(addr),
            request_channel: Some(Arc::new(Mutex::new(request_channel))),
            message_source: Some(message_source),
            runner: Some(runner),
            canceller: Some(canceller),
            servers: Some(HashMap::new()),
        }
    }

    pub fn register<T>(&mut self)
    where
        T: game_server::SupportedGame + game_server::GameServer + game_server::GameServerBuilder,
    {
        match T::get_config() {
            Some(config) => {
                self.servers
                    .as_mut()
                    .unwrap()
                    .insert(T::id(), T::build(config));
                println!("{} server registered", T::display_name());
            }
            None => println!("{} not configured, skipping.", T::display_name()),
        }
    }

    pub async fn run(mut self) {
        let servers = Arc::new(Mutex::new(self.servers.take().unwrap()));
        let ws_channel = self.request_channel.take().unwrap();

        let command_task = {
            let servers = servers.clone();
            ws_channel.lock().unwrap().on_request(move |message| {
                let servers = servers.clone();
                async move {
                    let target = message
                        .chars()
                        .take_while(|c| !c.is_whitespace())
                        .collect::<String>();
                    let rest = message.chars().skip(target.len()).collect::<String>();

                    match servers.lock().unwrap().get_mut(target.as_str()) {
                        Some(server) => server.handle_request(rest.trim()),
                        None => format!("No server of type {target}"),
                    }
                }
            })
        };

        let message_task = {
            let mut message_source = self.message_source.take().unwrap();
            let servers = servers.clone();
            async move {
                loop {
                    let message = match message_source.recv().await {
                        Some(message) => message.into_message(),
                        None => break,
                    };

                    let target = message
                        .chars()
                        .take_while(|c| !c.is_whitespace())
                        .collect::<String>();
                    let rest = message.chars().skip(target.len()).collect::<String>();

                    if let Some(server) = servers.lock().unwrap().get_mut(target.as_str()) {
                        server.handle_message(rest.trim());
                    }
                }
            }
        };

        let cancel_task = {
            let mut canceller = self.canceller.take();
            let runner = self.runner.take().unwrap();
            let addr = self.addr.take().unwrap();
            async move {
                ctrlc::set_handler(move || {
                    println!("^C");
                    if let Some(canceller) = canceller.take() {
                        canceller.cancel();
                    }
                })
                .expect("Error setting Ctrl+C handler");

                runner.run(addr).await;
            }
        };

        tokio::select! {
            _ = command_task => {},
            _ = tokio::spawn(message_task) => {},
            _ = tokio::spawn(cancel_task) => {},
        }
    }
}
