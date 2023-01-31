mod bot;
use bot::*;

pub async fn run() {
    let ws_server_addr =
        std::env::var("BJORN_WS_CONNECT_ADDRESS").expect("WS client env not configured");

    Bot::new()
        .expect("Error creating new bot.")
        .with_game::<minecraft::DiscordConfig>()
        .start(ws_server_addr)
        .await
        .expect("Error starting serenity Discord client.");
}
