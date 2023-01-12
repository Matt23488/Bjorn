use std::env;

use game_server::Dispatcher;
use serenity::framework::standard::macros::command;
use serenity::model::channel::Message;
use serenity::{framework::standard::CommandResult, prelude::*};

// TODO: An attribute macro that will decorate a fn and supply it with the Dispatcher
#[command]
pub async fn start(ctx: &Context, msg: &Message) -> CommandResult {
    if !validate_message(msg) {
        return Ok(())
    }

    let data = ctx.data.read().await;

    // * NOTE: Using this if let expression lets us ensure that `data` will not be used after any additional awaits.
    // * Otherwise the `command` macro errors.
    let ws_closed = if let Some(ws) = data.get::<Dispatcher>().unwrap().lock().unwrap().as_ref() {
        ws.dispatch("minecraft start".into()).is_err()
    } else {
        true
    };

    msg.reply(
        ctx,
        if ws_closed {
            "No connection to Game Manager."
        } else {
            "Starting Minecraft server..."
        },
    )
    .await?;

    Ok(())
}

#[command]
pub async fn stop(ctx: &Context, msg: &Message) -> CommandResult {
    if !validate_message(msg) {
        return Ok(())
    }

    let data = ctx.data.read().await;

    // * NOTE: Using this if let expression lets us ensure that `data` will not be used after any additional awaits.
    // * Otherwise the `command` macro errors.
    let ws_closed = if let Some(ws) = data.get::<Dispatcher>().unwrap().lock().unwrap().as_ref() {
        ws.dispatch("minecraft stop".into()).is_err()
    } else {
        true
    };

    msg.reply(
        ctx,
        if ws_closed {
            "No connection to Game Manager."
        } else {
            "Stopping Minecraft server..."
        },
    )
    .await?;

    Ok(())
}

#[command]
pub async fn save(ctx: &Context, msg: &Message) -> CommandResult {
    if !validate_message(msg) {
        return Ok(())
    }

    let data = ctx.data.read().await;

    // * NOTE: Using this if let expression lets us ensure that `data` will not be used after any additional awaits.
    // * Otherwise the `command` macro errors.
    let ws_closed = if let Some(ws) = data.get::<Dispatcher>().unwrap().lock().unwrap().as_ref() {
        ws.dispatch("minecraft save".into()).is_err()
    } else {
        true
    };

    msg.reply(
        ctx,
        if ws_closed {
            "No connection to Game Manager."
        } else {
            "Saving Minecraft server..."
        },
    )
    .await?;

    Ok(())
}

#[command]
pub async fn say(ctx: &Context, msg: &Message) -> CommandResult {
    if !validate_message(msg) {
        return Ok(())
    }

    // TODO: This number depends on prefix and command name. Need a way to look that up. Probably with another macro.
    if msg.content.len() < 5 {
        msg.reply(ctx, "You have to specify a message.").await?;

        return Ok(());
    }

    let data = ctx.data.read().await;

    // * NOTE: Using this if let expression lets us ensure that `data` will not be used after any additional awaits.
    // * Otherwise the `command` macro errors.
    let ws_closed = if let Some(ws) = data.get::<Dispatcher>().unwrap().lock().unwrap().as_ref() {
        ws.dispatch(format!("minecraft say {}", msg.content[5..].trim()))
            .is_err()
    } else {
        true
    };

    msg.reply(
        ctx,
        if ws_closed {
            "No connection to Game Manager."
        } else {
            "Sending chat to Minecraft server..."
        },
    )
    .await?;

    Ok(())
}

#[command]
pub async fn tp(ctx: &Context, msg: &Message) -> CommandResult {
    if !validate_message(msg) {
        return Ok(())
    }

    // TODO: This number depends on prefix and command name. Need a way to look that up. Probably with another macro.
    if msg.content.len() < 4 {
        msg.reply(ctx, "You have to specify some arguments.")
            .await?;

        return Ok(());
    }

    let data = ctx.data.read().await;

    // * NOTE: Using this if let expression lets us ensure that `data` will not be used after any additional awaits.
    // * Otherwise the `command` macro errors.
    let ws_closed = if let Some(ws) = data.get::<Dispatcher>().unwrap().lock().unwrap().as_ref() {
        ws.dispatch(format!("minecraft tp {}", msg.content[4..].trim()))
            .is_err()
    } else {
        true
    };

    msg.reply(
        ctx,
        if ws_closed {
            "No connection to Game Manager."
        } else {
            "Sending tp command Minecraft server..."
        },
    )
    .await?;

    Ok(())
}

fn validate_message(msg: &Message) -> bool {
    let channel_ok = match env::var("BJORN_MINECRAFT_DISCORD_COMMAND_CHANNEL") {
        Err(_) => false,
        Ok(channel) => match channel.parse::<u64>() {
            Ok(channel) => channel == msg.channel_id.0,
            Err(_) => false,
        }
    };

    let user_ok = match env::var("BJORN_MINECRAFT_DISCORD_ADMIN") {
        Err(_) => false,
        Ok(admin) => match admin.parse::<u64>() {
            Ok(admin) => admin == msg.author.id.0,
            Err(_) => false,
        }
    };

    channel_ok && user_ok
}
