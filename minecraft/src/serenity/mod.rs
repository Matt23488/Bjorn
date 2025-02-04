use bjorn_macro::bjorn_command;
use serenity::{
    cache::FromStrAndCache,
    framework::standard::CommandResult,
    model::prelude::{Mention, Message, UserId},
    prelude::*,
    utils::Color,
};

use discord_config::{use_data, DiscordGame};

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
pub async fn mstart(ctx: &Context, _: &Message) -> CommandResult {
    dispatch(ctx, server::Message::Start).await
}

#[bjorn_command(DiscordConfig, admin)]
pub async fn mstop(ctx: &Context, _: &Message) -> CommandResult {
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
                "You must register your Minecraft username with `!mplayer <username>` first.",
            )
            .await?;
            return Ok(());
        }
    };

    match command_args!(msg.content) {
        [] => send_tp_help_text(ctx, msg).await,
        ["set", "list"] => list_saved_locations(ctx, msg).await,
        ["set", name, realm, x, y, z] => save_tp_location(ctx, msg, name, realm, x, y, z, false).await,
        ["set", name, realm, x, y, z, "force"] => save_tp_location(ctx, msg, name, realm, x, y, z, true).await,
        ["set", ..] => send_tp_set_help_text(ctx, msg).await,
        [target, ..] => teleport(ctx, msg, name, target).await,
    }
}

#[bjorn_command(DiscordConfig)]
pub async fn mplayer(ctx: &Context, msg: &Message) -> CommandResult {
    match command_args!(msg.content) {
        [] => echo_player_name(ctx, msg).await,
        [name, ..] => register_player_name(ctx, msg, *name).await,
    }
}

#[bjorn_command(DiscordConfig, admin)]
pub async fn messages(ctx: &Context, msg: &Message) -> CommandResult {
    match command_args!(msg.content) {
        [value @ ("on" | "off")] => {
            let enabled = match value {
                &"on" => true,
                _ => false,
            };

            use_data!(ctx.data, mut |config: DiscordConfig| {
                config.toggle_server_messages(enabled);
            });

            msg.reply(ctx, format!("Server messages toggled {value} successfully.")).await?;

            Ok(())
        },
        [..] => {
            let value = use_data!(ctx.data, |config: DiscordConfig| {
                match config.server_messages_enabled() {
                    true => "on",
                    false => "off",
                }
            });

            msg.reply(ctx, format!("Syntax: `!messages <on|off>`\nMessages currently toggled {value}")).await?;
            
            Ok(())
        },
    }
}

// #[bjorn_command(DiscordConfig, admin)]
// pub async fn backup(ctx: &Context, _: &Message) -> CommandResult {
//     dispatch(ctx, server::Message::BackupWorld).await
// }

#[bjorn_command(DiscordConfig, admin)]
pub async fn cmd(ctx: &Context, msg: &Message) -> CommandResult {
    let command_text = match command_args!(msg.content) {
        [args @ ..] => {
            let data = ctx.data.read().await;
            let players = data.get::<Players>().unwrap().lock().unwrap();
            
            args.iter().map(|arg| {
                match Mention::from_str(ctx, arg) {
                    Ok(Mention::User(UserId(user_id))) => players.get_registered_name(user_id).unwrap_or(String::from(*arg)),
                    _ => String::from(*arg),
                }
            }).collect::<Vec<_>>().join(" ")
        }
    };

    dispatch(ctx, server::Message::Command(command_text)).await
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
                    msg.reply(ctx, format!("{mention} does not have their Minecraft username registered. It would be cool if they would use `!mplayer <username>` to fix that.")).await?;
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

async fn list_saved_locations(ctx: &Context, msg: &Message) -> CommandResult {
    let locations = {
        let data = ctx.data.read().await;
        let tp_locations = data.get::<TpLocations>().unwrap().lock().unwrap();
        tp_locations.all()
    };

    msg.channel_id
        .send_message(ctx, |m| {
            m.embed(|e| {
                let e = e.title("Saved Locations").color(Color::ROSEWATER);

                locations.into_iter().fold(e, |e, loc| {
                    e.field(loc.name, format!("`{:?}`", loc.coords), false)
                })
            })
        })
        .await?;

    Ok(())
}

async fn send_tp_help_text(ctx: &Context, msg: &Message) -> CommandResult {
    msg.channel_id.send_message(ctx, |m| {
        m.embed(|e| {
            e.title("Teleporting to a player")
                .color(Color::BLITZ_BLUE)
                .description("To teleport to a player, the command must match this format:")
                .field("format", "`!tp <player>`", false)
                .field("player", "Either the in-game player name you wish to teleport to (case-sensitive), or if they have registered themselves with the `!mplayer` command, their discord mention.", true)
        })
        .add_embed(|e| {
            e.title("Teleporting to a saved location")
                .color(Color::BLURPLE)
                .description("To teleport to a saved location, the command must match this format:")
                .field("format", "`!tp $<location>`", false)
                .field("location", "The name of the saved location prefixed with a dollar-sign (e.g. `!tp $home`).", true)
        })
        .reference_message(msg)
    }).await?;

    send_tp_set_help_text(ctx, msg).await?;

    Ok(())
}

async fn send_tp_set_help_text(ctx: &Context, msg: &Message) -> CommandResult {
    msg.channel_id.send_message(ctx, |m| {
        m.embed(|e| {
            e.title("Setting a saved location")
                .color(Color::FADED_PURPLE)
                .description("To set a saved location, the command must match this format:")
                .field("format", "`!tp set <name> <dimension> <x> <y> <z> [force]`", false)
                .field("name", "An alphanumeric name to give to the location.", true)
                .field("dimension", "Which realm the location is contained in. `minecraft:overworld` for Overworld, `minecraft:the_nether` for Nether, etc.", true)
                .field("x y z", "The 3D coordinates to save, separated by spaces. To obtain these, press F3 in game and your current coordinates are near the top on the left side of the screen.", true)
                .field("[force]", "If provided (without the square brackets), will overwrite existing location saved with the same name.", true)
        })
        .add_embed(|e| {
            e.title("Listing saved locations")
                .color(Color::FOOYOO)
                .description("To list all saved locations, use the command with these arguments:")
                .field("format", "`!tp set list`", false)
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
    force: bool,
) -> CommandResult {
    let x = x.parse::<f64>();
    let y = y.parse::<f64>();
    let z = z.parse::<f64>();

    let (x, y, z) = match (x, y, z) {
        (Ok(x), Ok(y), Ok(z)) => (x, y, z),
        _ => return send_tp_set_help_text(ctx, msg).await,
    };

    let coords = RealmCoords::new(realm, x, y, z);

    let success = {
        let data = ctx.data.read().await;
        let mut tp_locations = data.get::<TpLocations>().unwrap().lock().unwrap();
        tp_locations.save_coords(String::from(name), coords.clone(), force)
    };

    let reply = match success {
        true => format!("Minecraft teleport location `{name}` saved as `{coords:?}`."),
        false => format!("Minecraft teleport location `{name}` is already registered. Add \"force\" at the end of the command to overwrite."),
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
