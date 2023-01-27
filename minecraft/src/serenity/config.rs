use std::sync::Arc;

use serenity::{
    async_trait, framework::standard::macros::group, model::prelude::Message, prelude::*,
};
use ws_protocol::serenity::GameConfig;

use super::*;

impl TypeMapKey for DiscordConfig {
    type Value = std::sync::Mutex<Option<DiscordConfig>>;
}

#[group]
#[commands(start, stop, save, tp, players, player)] // TODO: Macro to add commands in?
struct Minecraft;

pub struct MessageHandler;

#[async_trait]
impl ws_protocol::serenity::BjornMessageHandler for MessageHandler {
    type Handler = client::Handler;

    async fn client_message(
        ctx: &Context,
        msg: &Message,
    ) {
        let data = ctx.data.read().await;
        let api = data
            .get::<ws_protocol::WsClient<server::Api>>()
            .unwrap()
            .lock()
            .unwrap();
        api.send(server::Message::Chat(
            msg.author.name.clone(),
            msg.content.replace("\n", " "),
        ));
    }

    async fn server_message(
        data: Arc<tokio::sync::RwLock<TypeMap>>,
        http_and_cache: Arc<serenity::CacheAndHttp>,
        message: client::Message,
    ) {
        let message = {
            let data = data.read().await;
            let players = data.get::<Players>().unwrap().lock().unwrap();

            message.to_string(&players)
        };

        ws_protocol::use_data!(data, |config: DiscordConfig| {
            for channel in &config.chat_channels {
                http_and_cache
                    .cache
                    .channel(*channel)
                    .unwrap()
                    .id()
                    .send_message(http_and_cache.http.clone(), |msg| msg.content(&message))
                    .await
                    .unwrap();
            }
        });
    }
}

#[async_trait]
impl GameConfig for DiscordConfig {
    type Config = DiscordConfig;
    type MessageHandler = MessageHandler;
    type Api = server::Api;
    type ApiHandler = client::Handler;

    fn id() -> &'static str {
        "minecraft"
    }

    fn command_group() -> &'static serenity::framework::standard::CommandGroup {
        &MINECRAFT_GROUP
    }

    fn new(game_config: Self::Config) -> Self::Value {
        std::sync::Mutex::new(Some(game_config))
    }

    fn new_ws_clients(
        client: &serenity::Client,
    ) -> (
        ws_protocol::WsClientComponents<server::Api>,
        ws_protocol::WsClientHandlerComponents<client::Api, client::Handler>,
    ) {
        (
            ws_protocol::WsClient::<server::Api>::new(),
            ws_protocol::WsClientHandler::new(client::Handler::new(client)),
        )
    }

    async fn has_necessary_permissions(
        &self,
        ctx: &Context,
        msg: &Message,
        role: ws_protocol::serenity::Role,
    ) -> bool {
        let guild_id = match msg.guild_id {
            Some(guild_id) => guild_id,
            None => return false,
        };

        if !self.listen_channels.contains(&msg.channel_id.0) {
            return false;
        }

        self.roles.has_role(ctx, msg, guild_id, role).await
    }
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct DiscordConfig {
    roles: ws_protocol::serenity::RoleConfig,
    listen_channels: Vec<u64>,
    chat_channels: Vec<u64>,
}

impl DiscordConfig {
    pub fn is_chat_channel(&self, channel_id: serenity::model::prelude::ChannelId) -> bool {
        self.chat_channels.contains(&channel_id.0)
    }
}
