use std::sync::mpsc;
use std::sync::Arc;
use std::sync::Mutex;

use config::Environment;
use game_server::Dispatcher;
use serde::Deserialize;
use serde::Serialize;
use serenity::async_trait;
use serenity::framework::standard::macros::group;
use serenity::framework::standard::StandardFramework;
use serenity::model::prelude::ChannelId;
use serenity::prelude::*;

use serenity_ctrlc::Disconnector;
use serenity_ctrlc::Ext;

use ws_protocol::BjornWsClient;
use ws_protocol::BjornWsClientType;

use minecraft::serenity::*;

#[group]
#[commands(start, stop, save, say, tp)] // TODO: Macro to take in `General` and add this macro
struct General;

struct Handler;

#[derive(Serialize, Deserialize)]
struct Config {
    prefix: String,
    channel: u64,
}

#[async_trait]
impl EventHandler for Handler {}

#[tokio::main]
async fn main() {
    let ws_client = Arc::new(Mutex::new(Some(BjornWsClient::new(
        BjornWsClientType::Discord,
    ))));

    let (config, secrets) = match Environment::load::<Config>("config.json", "../secrets.json") {
        Some(env) => env,
        None => return,
    };

    let framework = StandardFramework::new()
        .configure(|c| {
            c.prefix(config.prefix)
                .allowed_channels(vec![ChannelId(config.channel)].into_iter().collect())
        })
        .group(&GENERAL_GROUP);

    let intents = GatewayIntents::non_privileged() | GatewayIntents::MESSAGE_CONTENT;
    let mut discord_client = {
        let ws_client = ws_client.clone();
        Client::builder(secrets.bot_token(), intents)
            .event_handler(Handler)
            .framework(framework)
            .await
            .expect("Error creating client")
            .ctrlc_with(move |dc| {
                let ws_client = ws_client.clone();
                async move {
                    println!("Ctrl+C detected");

                    println!("Closing WebSocket client");
                    if let Some(client) = ws_client.lock().unwrap().take() {
                        client.shutdown();
                    }

                    println!("Disconnecting Discord bot");
                    Disconnector::disconnect_some(dc).await;
                }
            })
            .expect("Error registering Ctrl+C handler")
    };

    let (sender, receiver) = mpsc::channel::<String>();

    let sender = Mutex::new(Some(Dispatcher::new(sender)));

    let mut data = discord_client.data.write().await;
    
    data.insert::<Dispatcher>(sender);
    drop(data);

    let receiver = Arc::new(Mutex::new(receiver));
    tokio::spawn(async move {
        loop {
            let message = match receiver.lock().unwrap().recv() {
                Ok(message) => message,
                Err(_) => break,
            };

            if let Some(ws) = ws_client.lock().unwrap().as_ref() {
                ws.send_message(message).unwrap_or_default();
            }
        }
    });

    // start listening for events by starting a single shard
    if let Err(why) = discord_client.start().await {
        println!("An error occurred while running the client: {:?}", why);
    }
}
