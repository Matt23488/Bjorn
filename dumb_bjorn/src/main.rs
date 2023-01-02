use std::fs;

use serde::Deserialize;
use serde::Serialize;
use serenity::async_trait;
use serenity::prelude::*;
use serenity::model::channel::Message;
use serenity::framework::standard::macros::{command, group};
use serenity::framework::standard::{StandardFramework, CommandResult};

#[group]
#[commands(mode)]
struct General;

struct Handler;

#[derive(Serialize, Deserialize, Debug)]
struct Config {
    bot_token: String,
    prefix: String,
}

#[async_trait]
impl EventHandler for Handler {}

#[tokio::main]
async fn main() {
    let config = fs::read_to_string("../config.json").expect("config file");
    let config: Config = serde_json::from_str(config.as_str()).expect("valid config json");


    let framework = StandardFramework::new()
        .configure(|c| c.prefix(config.prefix))
        .group(&GENERAL_GROUP);

    let intents = GatewayIntents::non_privileged() | GatewayIntents::MESSAGE_CONTENT;
    let mut client = Client::builder(config.bot_token, intents)
        .event_handler(Handler)
        .framework(framework)
        .await
        .expect("Error creating client");

    // start listening for events by starting a single shard
    if let Err(why) = client.start().await {
        println!("An error occurred while running the client: {:?}", why);
    }
}

// TODO: This will show if the main bot app is running, and allow for rebooting/killing it.
#[command]
async fn mode(ctx: &Context, msg: &Message) -> CommandResult {
    match msg.reply(ctx.http.as_ref(), &msg.content).await {
        Ok(_) => Ok(()),
        Err(e) => Err(Box::new(e)),
    }
}
