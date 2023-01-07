use config::Config;
use config::Environment;
use serenity::async_trait;
use serenity::framework::standard::macros::{command, group};
use serenity::framework::standard::{CommandResult, StandardFramework};
use serenity::model::channel::Message;
use serenity::prelude::*;

use serenity_ctrlc::Ext;

use ws_protocol::BjornWsClient;
use ws_protocol::BjornWsClientType;

#[group]
#[commands(ping)]
struct General;

struct Handler;

#[async_trait]
impl EventHandler for Handler {}

#[tokio::main]
async fn main() {
    let ws_client = BjornWsClient::new(BjornWsClientType::Discord);

    let (config, secrets) = match Environment::load::<Config>("config.json", "../secrets.json") {
        Some(env) => env,
        None => return,
    };

    let framework = StandardFramework::new()
        .configure(|c| c.prefix(config.prefix()))
        .group(&GENERAL_GROUP);

    let intents = GatewayIntents::non_privileged() | GatewayIntents::MESSAGE_CONTENT;
    let mut client = Client::builder(secrets.bot_token(), intents)
        .event_handler(Handler)
        .framework(framework)
        .await
        .expect("Error creating client")
        .ctrlc()
        .expect("Error registering Ctrl+C handler");

    // start listening for events by starting a single shard
    if let Err(why) = client.start().await {
        println!("An error occurred while running the client: {:?}", why);
    }

    ws_client.shutdown();
}

#[command]
async fn ping(ctx: &Context, msg: &Message) -> CommandResult {
    msg.reply(ctx, "Pong!").await?;

    Ok(())
}
