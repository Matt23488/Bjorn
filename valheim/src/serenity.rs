use std::sync::Arc;

use bjorn_macro::bjorn_command;
use serenity::{framework::standard::CommandResult, model::prelude::*, prelude::*, utils::Color};

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
pub async fn vstart(ctx: &Context, msg: &Message) -> CommandResult {
    let msg = match command_args!(msg.content) {
        ["crossplay"] => server::Message::Start(true),
        [..] => server::Message::Start(false),
    };

    dispatch(ctx, msg).await
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

macro_rules! with_mention {
    ($players:expr, $player:expr) => {{
        $players
            .get_user_id($player)
            .map(|id| Mention::User(UserId(id)).to_string())
            .unwrap_or($player.clone())
    }};
}

impl client::Message {
    pub async fn send_discord_message<'m>(&self, players: &Players, http: &Arc<serenity::http::Http>, channel: &ChannelId) -> Result<Message, SerenityError> {
        match self {
            Self::StartupBegin => channel.say(http, "Valheim Server starting...").await,
            Self::StartupComplete(None) => channel.say(http, "Valheim Server startup complete.").await,
            Self::StartupComplete(Some(code)) => {
                channel.say(http, format!("Valheim Server startup complete. Join Code is `{code}`.")).await
            }
            Self::ShutdownBegin => channel.say(http, "Valheim Server shutting down...").await,
            Self::ShutdownComplete => channel.say(http, "Valheim Server shutdown complete.").await,
            Self::Info(message) => channel.say(http, format!("[Info] {message}")).await,
            Self::PlayerJoined(player) => {
                channel.say(http, format!("{} joined the server!", with_mention!(players, player))).await
            }
            Self::PlayerQuit(player) => {
                channel.say(http, format!("{} left the server.", with_mention!(players, player))).await
            }
            Self::PlayerDied(player) => {
                channel.say(http, format!("{} died.", with_mention!(players, player))).await
            }
            Self::Haldor(locations) => {
                if locations.is_empty() {
                    channel.say(http, String::from("No locations returned. Probably a bug.")).await
                } else {
                    channel.send_message(http, |msg| {
                        msg.embed(move |e| e
                            .color(Color::ROHRKATZE_BLUE)
                            .title(if locations.len() > 1 {
                                "Possible Haldor Locations"
                            } else {
                                "Haldor Location"
                            })
                            .description(if locations.len() > 1 {
                                "Haldor has not been found on this world. Locations are relative to the center of the map."
                            } else {
                                "Haldor has been found. Location is relative to the center of the map."
                            })
                            .fields(locations.into_iter()
                                .map(|(x, z)| (
                                    (x*x + z*z).sqrt(),
                                    format!("{}m {}", x.abs().round(), if *x < 0.0 { 'W' } else { 'E' }),
                                    format!("{}m {}", z.abs().round(), if *z < 0.0 { 'S' } else { 'N' }),
                                ))
                                .map(|(dist, x, z)| (
                                    format!("{dist}m away"),
                                    format!("```{x}\n{z}```"),
                                    true,
                                )
                            ))
                        )
                    }).await
                }
            }
            Self::MobAttack(event_id) => channel.say(http, format!("We're under attack! No fancy embeds yet, but the id is `{event_id}`.")).await,
        }
    }
}
