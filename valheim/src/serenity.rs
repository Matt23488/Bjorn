use bjorn_macro::bjorn_command;
use serenity::{framework::standard::CommandResult, model::prelude::*, prelude::*};

use discord_config::DiscordGame;

use crate::{client, server};

mod config;
pub use self::config::*;

mod json;
pub use json::*;

mod players;
pub use players::*;

macro_rules! command_args {
    ($text:expr) => {
        $text
            .split_whitespace()
            .skip(1)
            .collect::<Vec<_>>()
            .as_slice()
    };
}

#[bjorn_command(DiscordConfig)]
pub async fn vstart(ctx: &Context, _: &Message) -> CommandResult {
    dispatch(ctx, server::Message::Start).await
}

#[bjorn_command(DiscordConfig, admin)]
pub async fn vstop(ctx: &Context, _: &Message) -> CommandResult {
    dispatch(ctx, server::Message::Stop).await
}

#[bjorn_command(DiscordConfig)]
pub async fn vplayer(ctx: &Context, msg: &Message) -> CommandResult {
    match command_args!(msg.content) {
        [] => echo_player_name(ctx, msg).await,
        [name, ..] => register_player_name(ctx, msg, *name).await,
    }
}

#[bjorn_command(DiscordConfig)]
pub async fn haldor(ctx: &Context, _msg: &Message) -> CommandResult {
    dispatch(ctx, server::Message::QueryHaldor).await
}

async fn echo_player_name(ctx: &Context, msg: &Message) -> CommandResult {
    let name = {
        let data = ctx.data.read().await;
        let players = data.get::<Players>().unwrap().lock().unwrap();
        players.get_registered_name(msg.author.id.0)
    };

    let reply = match name {
        Some(name) => format!("Your registered Valheim username is `{name}`."),
        None => String::from("You don't currently have a Valheim username registered."),
    };

    msg.reply(ctx, reply).await?;
    Ok(())
}

async fn register_player_name(ctx: &Context, msg: &Message, name: &str) -> CommandResult {
    let success = {
        let data = ctx.data.read().await;
        let mut players = data.get::<Players>().unwrap().lock().unwrap();
        players.set_player_name(msg.author.id.0, String::from(name))
    };

    let reply = match success {
        true => format!("Valheim username `{name}` registered to you."),
        false => format!("Valheim username `{name}` is already registered."),
    };

    msg.reply(ctx, reply).await?;
    Ok(())
}

async fn dispatch(ctx: &Context, message: server::Message) -> CommandResult {
    let data = ctx.data.read().await;

    let api = data
        .get::<ws_protocol::WsClient<server::Api>>()
        .unwrap()
        .lock()
        .unwrap();

    api.send(message);

    Ok(())
}
