use bjorn_macro::bjorn_command;
use serenity::{framework::standard::CommandResult, model::prelude::{Message, Mention, UserId}, prelude::*, cache::FromStrAndCache};

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
    let name = {
        let data = ctx.data.read().await;
        let players = data.get::<Players>().unwrap().lock().unwrap();
        players.get_registered_name(msg.author.id.0)
    };

    let name = match name {
        Some(name) => name,
        None => {
            msg.reply(ctx, "You must register your Minecraft username with !player _username_ first.").await.unwrap();
            return Ok(());
        }
    };

    let msg_parts = msg.content.split_whitespace().skip(1).collect::<Vec<_>>();

    if msg_parts.is_empty() {
        msg.reply(ctx, "You must specify a player to teleport to.").await.unwrap();
        return Ok(());
    }

    let target = match Mention::from_str(ctx, msg_parts.get(0).unwrap()) {
        Ok(mention @ Mention::User(UserId(user_id))) => {
            let user = {
                let data = ctx.data.read().await;
                let players = data.get::<Players>().unwrap().lock().unwrap();
                players.get_registered_name(user_id)
            };

            match user {
                Some(user) => user,
                None => {
                    msg.reply(ctx, format!("{mention} does not have their Minecraft username registered. It would be cool if they would use **!player _username_** to fix that.")).await.unwrap();
                    return Ok(());
                }
            }
        }
        _ => {
            String::from(*msg_parts.get(0).unwrap())
        }
    };

    if name == target {
        msg.reply(ctx, "You can't teleport to yourself.").await.unwrap();
        return Ok(());
    }

    let message = server::Message::Tp(name, target);
    dispatch(ctx, message).await
}

#[bjorn_command(DiscordConfig)]
pub async fn player(ctx: &Context, msg: &Message) -> CommandResult {
    let msg_parts = msg.content.split_whitespace().skip(1).collect::<Vec<_>>();

    if msg_parts.len() < 1 {
        let name = {
            let data = ctx.data.read().await;
            let players = data.get::<Players>().unwrap().lock().unwrap();
            players.get_registered_name(msg.author.id.0)
        };

        let reply = if name.is_none() {
            String::from("You don't currently have any Minecraft usernames registered.")
        } else {
            format!("Your registered Minecraft username is _{}_.", name.unwrap())
        };

        msg.reply(ctx, reply).await.unwrap();
        return Ok(());
    }

    let name = *msg_parts.get(1).unwrap();

    let success = {
        let data = ctx.data.read().await;
        let mut players = data.get::<Players>().unwrap().lock().unwrap();
        players.set_player_name(msg.author.id.0, String::from(name))
    };

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
