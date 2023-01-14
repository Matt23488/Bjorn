use std::env;

use serenity::async_trait;
use serenity::framework::standard::macros::group;
use serenity::framework::standard::StandardFramework;
use serenity::prelude::*;

use minecraft::*;
use ws_protocol::client::ClientExtension;

#[group]
#[commands(start, stop, save, say, tp)] // TODO: Macro to add commands in?
struct General;

struct Handler;

#[async_trait]
impl EventHandler for Handler {}

#[tokio::main]
async fn main() {
    let (prefix, bot_token) = match (
        env::var("BJORN_DISCORD_PREFIX"),
        env::var("BJORN_DISCORD_TOKEN"),
    ) {
        (Ok(prefix), Ok(token)) => (prefix, token),
        _ => panic!("Discord environment not configured"),
    };

    let framework = StandardFramework::new()
        .configure(|c| c.prefix(prefix))
        .group(&GENERAL_GROUP);

    let intents = GatewayIntents::non_privileged() | GatewayIntents::MESSAGE_CONTENT;

    Client::builder(bot_token, intents)
        .event_handler(Handler)
        .framework(framework)
        .await
        .expect("Error creating Discord client")
        .start_with_bjorn()
        .await
        .expect("Error starting Discord client");
}
