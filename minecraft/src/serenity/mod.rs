use bjorn_macro::bjorn_command;
use serenity::{
    cache::FromStrAndCache,
    framework::standard::CommandResult,
    model::prelude::{Mention, Message, UserId},
    prelude::*,
};

use discord_config::GameConfig;

use crate::{
    client,
    server::{self, RealmCoords},
};

mod config;
pub use self::config::*;

mod json;
pub use json::*;

mod players;
pub use players::*;

mod tp_locations;
pub use tp_locations::*;

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
            msg.reply(
                ctx,
                "You must register your Minecraft username with `!player <username>` first.",
            )
            .await?;
            return Ok(());
        }
    };

    match command_args!(msg.content) {
        [] => {
            // TODO: embed
            msg.reply(ctx, "You must specify a player or saved location to teleport to, or `!tp set <name> <o|n|e> <x> <y> <z>` to create a saved location.").await?;
            Ok(())
        }
        ["set", name, realm, x, y, z] => save_tp_location(ctx, msg, name, realm, x, y, z).await,
        ["set", ..] => send_tp_set_help_text(ctx, msg).await,
        [target, ..] => teleport(ctx, msg, name, target).await,
    }
}

#[bjorn_command(DiscordConfig)]
pub async fn player(ctx: &Context, msg: &Message) -> CommandResult {
    match command_args!(msg.content) {
        [] => echo_player_name(ctx, msg).await,
        [name, ..] => register_player_name(ctx, msg, *name).await,
    }
}

async fn teleport(ctx: &Context, msg: &Message, player: String, target: &str) -> CommandResult {
    if &target[0..1] == "$" {
        return tp_saved_location(ctx, msg, player, &target[1..]).await;
    }

    let target = match Mention::from_str(ctx, target) {
        Ok(mention @ Mention::User(UserId(user_id))) => {
            let target = {
                let data = ctx.data.read().await;
                let players = data.get::<Players>().unwrap().lock().unwrap();
                players.get_registered_name(user_id)
            };

            match target {
                Some(target) => target,
                None => {
                    msg.reply(ctx, format!("{mention} does not have their Minecraft username registered. It would be cool if they would use `!player <username>` to fix that.")).await?;
                    return Ok(());
                }
            }
        }
        _ => String::from(target),
    };

    if player == target {
        msg.reply(ctx, "You can't teleport to yourself.").await?;
        Ok(())
    } else {
        dispatch(ctx, server::Message::Tp(player, target)).await
    }
}

async fn tp_saved_location(
    ctx: &Context,
    msg: &Message,
    player: String,
    target: &str,
) -> CommandResult {
    let coords = {
        let data = ctx.data.read().await;
        let tp_locations = data.get::<TpLocations>().unwrap().lock().unwrap();
        tp_locations.get_coords(target)
    };

    let coords = match coords {
        Some(coords) => coords,
        None => {
            msg.reply(
                ctx,
                format!("No saved coordinates with name `{target}` exists."),
            )
            .await?;
            return Ok(());
        }
    };

    dispatch(ctx, server::Message::TpLoc(player, coords)).await
}

async fn send_tp_set_help_text(ctx: &Context, msg: &Message) -> CommandResult {
    msg.channel_id.send_message(ctx, |m| {
        m.embed(|e| {
            e.title("Setting a saved location")
                .description("To set a saved location, the command must match this format:")
                .field("format", "`!tp set <name> <o|n|e> <x> <y> <z>`", false)
                .field("name", "An alphanumeric name to give to the location.", true)
                .field("o|n|e", "Which realm the location is contained in. `o` for Overworld, `n` for Nether, or `e` for The End.", true)
                .field("x y z", "The 3D coordinates to save, separated by spaces. To obtain these, press F3 in game and your current coordinates are near the top on the left side of the screen.", true)
        })
        .reference_message(msg)
    }).await?;
    Ok(())
}

async fn save_tp_location(
    ctx: &Context,
    msg: &Message,
    name: &str,
    realm: &str,
    x: &str,
    y: &str,
    z: &str,
) -> CommandResult {
    let x = x.parse::<f64>();
    let y = y.parse::<f64>();
    let z = z.parse::<f64>();

    let (x, y, z) = match (x, y, z) {
        (Ok(x), Ok(y), Ok(z)) => (x, y, z),
        _ => return send_tp_set_help_text(ctx, msg).await,
    };

    let coords = match RealmCoords::new(realm, x, y, z) {
        Some(coords) => coords,
        None => return send_tp_set_help_text(ctx, msg).await,
    };

    let success = {
        let data = ctx.data.read().await;
        let mut tp_locations = data.get::<TpLocations>().unwrap().lock().unwrap();
        tp_locations.save_coords(String::from(name), coords)
    };

    let reply = match success {
        true => format!("Minecraft teleport location `{name}` saved as `{coords:?}`."),
        false => format!("Minecraft teleport location `{name}` is already registered."),
    };

    msg.reply(ctx, reply).await?;
    Ok(())
}

async fn echo_player_name(ctx: &Context, msg: &Message) -> CommandResult {
    let name = {
        let data = ctx.data.read().await;
        let players = data.get::<Players>().unwrap().lock().unwrap();
        players.get_registered_name(msg.author.id.0)
    };

    let reply = match name {
        Some(name) => format!("Your registered Minecraft username is `{name}`."),
        None => String::from("You don't currently have a Minecraft username registered."),
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
        true => format!("Minecraft username `{name}` registered to you."),
        false => format!("Minecraft username `{name}` is already registered."),
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
