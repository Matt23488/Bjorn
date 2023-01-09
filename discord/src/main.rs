use std::collections::HashMap;

use config::Config;
use config::Environment;
use serenity::async_trait;
use serenity::framework::standard::macros::{command, group};
use serenity::framework::standard::{CommandResult, StandardFramework};
use serenity::model::channel::Message;
use serenity::prelude::*;

use serenity_ctrlc::Disconnector;
use serenity_ctrlc::Ext;

use ws_protocol::BjornWsClient;
use ws_protocol::BjornWsClientType;

#[group]
#[commands(ping, ws)]
struct General;

struct Handler;

#[async_trait]
impl EventHandler for Handler {}

#[tokio::main]
async fn main() {
    let ws_client = std::sync::Arc::new(std::sync::Mutex::new(Some(BjornWsClient::new(
        BjornWsClientType::Discord,
    ))));

    let (config, secrets) = match Environment::load::<Config>("config.json", "../secrets.json") {
        Some(env) => env,
        None => return,
    };

    let framework = StandardFramework::new()
        .configure(|c| c.prefix(config.prefix()))
        .group(&GENERAL_GROUP);

    let intents = GatewayIntents::non_privileged() | GatewayIntents::MESSAGE_CONTENT;
    let mut client = {
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
                    println!("Disconnecting Discord bot");
                    Disconnector::disconnect_some(dc).await;

                    println!("Closing WebSocket client");
                    if let Some(client) = ws_client.lock().unwrap().take() {
                        client.shutdown();
                    }
                }
            })
            .expect("Error registering Ctrl+C handler")
    };

    let mut data = client.data.write().await;
    let mut map = HashMap::new();
    map.insert("ws".into(), ws_client);
    data.insert::<BjornWsClient>(map);
    drop(data);

    // start listening for events by starting a single shard
    if let Err(why) = client.start().await {
        println!("An error occurred while running the client: {:?}", why);
    }
}

#[command]
async fn ping(ctx: &Context, msg: &Message) -> CommandResult {
    msg.reply(ctx, "Pong!").await?;

    Ok(())
}

#[command]
async fn ws(ctx: &Context, msg: &Message) -> CommandResult {
    let data = ctx.data.read().await;

    if msg.content.len() < 4 {
        msg.reply(ctx, "You must specify a message to send.")
            .await?;
    } else {
        let ws_closed = if let Some(ws) = data
            .get::<BjornWsClient>()
            .unwrap()
            .get("ws")
            .unwrap()
            .lock()
            .unwrap()
            .as_ref()
        {
            ws.send_message(&msg.content[4..]).is_err()
        } else {
            true
        };

        if ws_closed {
            msg.reply(ctx, "WS connection closed.").await?;
        }
    }

    Ok(())
}
