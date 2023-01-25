use bjorn_macro::bjorn_command;
use serenity::{framework::standard::CommandResult, model::prelude::Message, prelude::*};

use ws_protocol::serenity::GameConfig;

mod config;
use crate::{client, server};

pub use self::config::*;

#[bjorn_command(DiscordConfig)]
pub async fn start(ctx: &Context, _: &Message) -> CommandResult {
    dispatch(ctx, server::Message::Start).await
}

#[bjorn_command(DiscordConfig, admin)]
pub async fn stop(ctx: &Context, _: &Message) -> CommandResult {
    dispatch(ctx, server::Message::Stop).await
}

#[bjorn_command(DiscordConfig)]
pub async fn save(ctx: &Context, _: &Message) -> CommandResult {
    dispatch(ctx, server::Message::Save).await
}

#[bjorn_command(DiscordConfig)]
pub async fn players(ctx: &Context, _: &Message) -> CommandResult {
    dispatch(ctx, server::Message::QueryPlayers).await
}

#[bjorn_command(DiscordConfig)]
pub async fn tp(ctx: &Context, msg: &Message) -> CommandResult {
    if msg.content.len() < 4 {
        msg.reply(ctx, "You must specify a message.").await.unwrap();

        return Ok(());
    }

    let rest = msg
        .content
        .chars()
        .skip(4)
        .skip_while(|c| c.is_whitespace())
        .collect::<String>();
    let rest = rest.split_whitespace().collect::<Vec<_>>();

    if rest.len() < 2 {
        msg.reply(
            ctx,
            "You must specify both a player to teleport and a target.",
        )
        .await
        .unwrap();

        return Ok(());
    }

    let message = server::Message::Tp(
        (*rest.get(0).unwrap()).into(),
        (*rest.get(1).unwrap()).into(),
    );

    dispatch(ctx, message).await
}

// TODO: Maybe move into macro
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
