mod bot;
use bot::*;

pub async fn run() {
    let ws_server_addr =
        std::env::var("BJORN_WS_CONNECT_ADDRESS").expect("WS client env not configured");

    Bot::new()
        .expect("Error creating new bot.")
        // .with_config::<minecraft::DiscordConfig>() // TODO: I will figure out a way to make this work later.
        .init()
        .await
        .expect("Bjorn bot setup failed.")
        .start(&ws_server_addr)
        .await
        .expect("Error starting serenity Discord client.");
}
