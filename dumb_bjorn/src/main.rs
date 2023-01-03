use config::Environment;
use serenity::async_trait;
use serenity::prelude::*;
use serenity::model::channel::Message;
use serenity::framework::standard::macros::{command, group};
use serenity::framework::standard::{StandardFramework, CommandResult};

mod dumb_config;
use dumb_config::DumbConfig;

#[group]
#[commands(mode)]
struct General;

struct Handler;

#[async_trait]
impl EventHandler for Handler {}

#[tokio::main]
async fn main() {
    let (config, secrets) = match Environment::load::<DumbConfig>("config.json", "../secrets.json") {
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
        .expect("Error creating client");

    // start listening for events by starting a single shard
    if let Err(why) = client.start().await {
        println!("An error occurred while running the client: {:?}", why);
    }
}

// TODO: logic for managing the smart mode binary
#[command]
async fn mode(ctx: &Context, msg: &Message) -> CommandResult {
    let args = msg.content.split_ascii_whitespace().skip(1).collect::<Vec<_>>();
    if args.len() < 1 {
        return Ok(());
    }

    let text = match args[0] {
        "d" | "dumb" => "dumb mode",
        "s" | "smart" => "smart mode",
        "r" | "reboot" => "reboot smart mode process",
        _ => "unknown arg",
    };

    if let Err(e) = msg.reply(ctx.http.as_ref(), text).await {
        eprintln!("MODE: Couldn't reply. Error: {e}");
    }

    Ok(())
}
