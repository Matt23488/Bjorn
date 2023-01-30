use std::sync::{Arc, Mutex};

use discord_config::{use_data, GameConfig};
use serenity::{
    async_trait, framework::standard::macros::group, model::prelude::Message, prelude::*, http::Typing,
};

use super::*;

impl TypeMapKey for DiscordConfig {
    type Value = std::sync::Mutex<Option<DiscordConfig>>;
}

#[group]
#[commands(start, stop, save, tp, players, player)] // TODO: Macro to add commands in?
struct Minecraft;

pub struct MessageHandler;

#[async_trait]
impl discord_config::BjornMessageHandler for MessageHandler {
    type Handler = client::Handler;

    async fn client_message(ctx: &Context, msg: &Message) {
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
        let (has_follow_up, message) = {
            let data = data.read().await;
            let players = data.get::<Players>().unwrap().lock().unwrap();

            (
                message.indicates_follow_up(),
                message.to_string(&players),
            )
        };

        use_data!(data, |config: DiscordConfig| {
            let mut typing_results = vec![];
            for channel in &config.chat_channels {
                let channel = http_and_cache
                    .cache
                    .channel(*channel)
                    .unwrap()
                    .id();

                channel
                    .send_message(http_and_cache.http.clone(), |msg| msg.content(&message))
                    .await
                    .unwrap();

                if has_follow_up {
                    if let Ok(typing) = channel.start_typing(&http_and_cache.http) {
                        typing_results.push(typing);
                    }
                }
            }

            let mut data = data.write().await;
            if has_follow_up {
                data.insert::<TypingResults>(Arc::new(Mutex::new(Some(TypingResults(typing_results)))));
            } else {
                if let Some(typing_results) = data.remove::<TypingResults>() {
                    if let Some(typing_results) = typing_results.lock().unwrap().take() {
                        typing_results.0
                            .into_iter()
                            .map(Typing::stop)
                            .for_each(Option::unwrap_or_default);
                    }
                }
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
        role: discord_config::Role,
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
    roles: discord_config::RoleConfig,
    listen_channels: Vec<u64>,
    chat_channels: Vec<u64>,
}

impl DiscordConfig {
    pub fn is_chat_channel(&self, channel_id: serenity::model::prelude::ChannelId) -> bool {
        self.chat_channels.contains(&channel_id.0)
    }
}

struct TypingResults(Vec<Typing>);

impl TypeMapKey for TypingResults {
    type Value = Arc<Mutex<Option<TypingResults>>>;
}
