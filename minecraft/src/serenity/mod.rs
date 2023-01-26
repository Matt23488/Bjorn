use bjorn_macro::bjorn_command;
use serenity::{framework::standard::CommandResult, model::prelude::Message, prelude::*};

use ws_protocol::serenity::GameConfig;

use crate::{client, server};

mod config;
pub use self::config::*;

mod players;
pub use players::*;

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

#[bjorn_command(DiscordConfig)]
pub async fn player(ctx: &Context, msg: &Message) -> CommandResult {
    let msg_parts = msg.content.split_whitespace().collect::<Vec<_>>();

    if msg_parts.len() < 2 {
        let data = ctx.data.read().await;
        let players = data.get::<Players>().unwrap();
        let name_list = players
            .lock()
            .unwrap()
            .get_registered_names(msg.author.id.0)
            .iter()
            .map(|name| format!("_{name}_"))
            .collect::<Vec<_>>()
            .join(", ");
        drop(data);

        let reply = if name_list.is_empty() {
            String::from("You don't currently have any Minecraft usernames registered.")
        } else {
            format!("Your registered Minecraft usernames are: {name_list}")
        };
        
        msg.reply(ctx, reply).await.unwrap();
        return Ok(());
    }

    let name = *msg_parts.get(1).unwrap();

    let data = ctx.data.read().await;
    let players = data.get::<Players>().unwrap();
    let success = players
        .lock()
        .unwrap()
        .add_player_name(msg.author.id.0, String::from(name));
    drop(data);

    let reply = match success {
        true => format!("Minecraft username _{name}_ registered to you."),
        false => format!("Minecraft username _{name}_ is already registered."),
    };

    msg.reply(ctx, reply).await.unwrap();

    Ok(())
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
