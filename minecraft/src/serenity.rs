use serenity::{
    framework::standard::{macros::command, CommandResult},
    model::prelude::Message,
    prelude::*,
};

#[command]
pub async fn start(ctx: &Context, msg: &Message) -> CommandResult {
    dispatch(
        ctx,
        msg,
        ws_protocol::WsClientType::ServerManager,
        "minecraft start".into(),
    )
    .await
}

#[command]
pub async fn stop(ctx: &Context, msg: &Message) -> CommandResult {
    dispatch(
        ctx,
        msg,
        ws_protocol::WsClientType::ServerManager,
        "minecraft stop".into(),
    )
    .await
}

#[command]
pub async fn save(ctx: &Context, msg: &Message) -> CommandResult {
    dispatch(
        ctx,
        msg,
        ws_protocol::WsClientType::ServerManager,
        "minecraft save".into(),
    )
    .await
}

#[command]
pub async fn say(ctx: &Context, msg: &Message) -> CommandResult {
    if msg.content.len() < 5 {
        msg.reply(ctx, "You must specify a message.").await.unwrap();

        return Ok(());
    }

    let message = format!(
        "minecraft say {}",
        msg.content
            .chars()
            .skip(5)
            .skip_while(|c| c.is_whitespace())
            .collect::<String>()
    );

    dispatch(ctx, msg, ws_protocol::WsClientType::ServerManager, message).await
}

#[command]
pub async fn tp(ctx: &Context, msg: &Message) -> CommandResult {
    if msg.content.len() < 4 {
        msg.reply(ctx, "You must specify a message.").await.unwrap();

        return Ok(());
    }

    let message = format!(
        "minecraft tp {}",
        msg.content
            .chars()
            .skip(4)
            .skip_while(|c| c.is_whitespace())
            .collect::<String>()
    );

    dispatch(ctx, msg, ws_protocol::WsClientType::ServerManager, message).await
}

async fn dispatch(
    ctx: &Context,
    msg: &Message,
    recipient: ws_protocol::WsClientType,
    text: String,
) -> CommandResult {
    let data = ctx.data.read().await;

    // Ignore if config is None.
    let config = match data
        .get::<ws_protocol::client::DiscordConfig>()
        .unwrap()
        .lock()
        .unwrap()
        .take()
    {
        Some(config) => config,
        None => return Ok(()),
    };

    // Ignore if wrong channel or user.
    if config.channel_id() != msg.channel_id.0 || config.user_id() != msg.author.id.0 {
        return Ok(());
    }

    data.get::<ws_protocol::client::DiscordConfig>()
        .unwrap()
        .lock()
        .unwrap()
        .replace(config);

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

    let reply = ws_channel.request(recipient, text).await;

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
