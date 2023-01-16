use game_server_macro::bjorn_command;
use serenity::{
    framework::standard::{macros::command, CommandResult},
    model::prelude::Message,
    prelude::*,
};

use ws_protocol::client::GameConfigTypeMapKey;

mod config;
pub use self::config::*;

#[bjorn_command(DiscordConfig)]
pub async fn start(ctx: &Context, msg: &Message) -> CommandResult {
    dispatch(ctx, msg, "start".into()).await
}

#[bjorn_command(DiscordConfig, admin)]
pub async fn stop(ctx: &Context, msg: &Message) -> CommandResult {
    dispatch(ctx, msg, "stop".into()).await
}

#[bjorn_command(DiscordConfig)]
pub async fn save(ctx: &Context, msg: &Message) -> CommandResult {
    dispatch(ctx, msg, "save".into()).await
}

#[bjorn_command(DiscordConfig, admin)]
pub async fn say(ctx: &Context, msg: &Message) -> CommandResult {
    if msg.content.len() < 5 {
        msg.reply(ctx, "You must specify a message.").await.unwrap();

        return Ok(());
    }

    let message = format!(
        "say {}",
        msg.content
            .chars()
            .skip(5)
            .skip_while(|c| c.is_whitespace())
            .collect::<String>()
    );

    dispatch(ctx, msg, message.as_str()).await
}

#[bjorn_command(DiscordConfig)]
pub async fn tp(ctx: &Context, msg: &Message) -> CommandResult {
    if msg.content.len() < 4 {
        msg.reply(ctx, "You must specify a message.").await.unwrap();

        return Ok(());
    }

    let message = format!(
        "tp {}",
        msg.content
            .chars()
            .skip(4)
            .skip_while(|c| c.is_whitespace())
            .collect::<String>()
    );

    dispatch(ctx, msg, message.as_str()).await
}

// TODO: Maybe move into macro
async fn dispatch(ctx: &Context, msg: &Message, text: &str) -> CommandResult {
    let data = ctx.data.read().await;

    // Ignore if channel is None.
    let mut ws_channel = match data
        .get::<ws_protocol::WsChannel>()
        .unwrap()
        .lock()
        .unwrap()
        .take()
    {
        Some(channel) => channel,
        None => return Ok(()),
    };

    drop(data);

    let reply = ws_channel
        .request(
            ws_protocol::WsClientType::ServerManager,
            format!("minecraft {text}"),
        )
        .await;

    let data = ctx.data.read().await;
    data.get::<ws_protocol::WsChannel>()
        .unwrap()
        .lock()
        .unwrap()
        .replace(ws_channel);
    drop(data);

    msg.reply(ctx, reply).await.unwrap();

    Ok(())
}
