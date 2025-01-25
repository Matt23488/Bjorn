use std::sync::{Arc, Mutex};

use discord_config::use_data;
use serenity::{
    async_trait, framework::standard::macros::group, http::Typing, model::{id::WebhookId, prelude::Message, webhook::Webhook},
    prelude::*,
};
use ws_protocol::WsTask;

use super::*;

impl TypeMapKey for DiscordConfig {
    type Value = Mutex<Option<DiscordConfig>>;
}

#[group]
#[commands(mstart, mstop, save, tp, players, mplayer)] // TODO: Macro to add commands in?
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
        let (has_follow_up, message_text) = {
            let data = data.read().await;
            let players = data.get::<Players>().unwrap().lock().unwrap();

            (message.indicates_follow_up(), message.to_string(&players))
        };

        if let client::Message::Command(player, command, target) = message {
            let message = match command.as_str() {
                "tp" => {
                    if &target[0..1] == "$" {
                        let coords = {
                            let data = data.read().await;
                            let tp_locations = data.get::<TpLocations>().unwrap().lock().unwrap();
                            tp_locations.get_coords(&target[1..])
                        };

                        match coords {
                            Some(coords) => Some(server::Message::TpLoc(player, coords)),
                            None => None,
                        }
                    } else {
                        Some(server::Message::Tp(player, target))
                    }
                }
                _ => None,
            };

            if let Some(message) = message {
                let data = data.read().await;
                let server_api = data
                    .get::<ws_protocol::WsClient<server::Api>>()
                    .unwrap()
                    .lock()
                    .unwrap();
                server_api.send(message);
            }
        }

        use_data!(data, |config: DiscordConfig| {
            let mut typing_results = vec![];
            for channel in &config.chat_channels {
                let webhook_id = WebhookId(channel.id);
                let _webhook = Webhook::from_id_with_token(&http_and_cache.http, webhook_id, &channel.token);
                
                // TODO: Convert this to webhook call
                let channel = http_and_cache.cache.channel(channel.id).unwrap().id();

                channel
                    .send_message(http_and_cache.http.clone(), |msg| {
                        msg.content(&message_text)
                    })
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
                data.insert::<TypingResults>(Arc::new(Mutex::new(Some(TypingResults(
                    typing_results,
                )))));
            } else {
                if let Some(typing_results) = data.remove::<TypingResults>() {
                    if let Some(typing_results) = typing_results.lock().unwrap().take() {
                        typing_results
                            .0
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
impl discord_config::DiscordGame for DiscordConfig {
    fn id() -> &'static str {
        "minecraft"
    }

    fn command_group() -> &'static serenity::framework::standard::CommandGroup {
        &MINECRAFT_GROUP
    }

    fn setup(
        setup_data: discord_config::DiscordGameSetupData,
        serenity_data: &mut serenity::prelude::TypeMap,
        canceller: &mut discord_config::Canceller,
    ) -> Result<(), ()> {
        let discord_config::DiscordGameSetupData {
            config_path,
            data,
            cache_and_http,
            addr,
        } = setup_data;

        let config = match discord_config::load_config(&config_path) {
            Some(config) => config,
            None => return Err(()),
        };

        serenity_data.insert::<DiscordConfig>(Mutex::new(Some(config)));

        let (server_api, runner, out_canceller) = ws_protocol::WsClient::<server::Api>::new();
        let (client_handler, in_canceller) =
            ws_protocol::WsClientHandler::new(client::Handler::new(data, cache_and_http));

        canceller.add(out_canceller);
        canceller.add(in_canceller);

        serenity_data.insert::<ws_protocol::WsClient<server::Api>>(Mutex::new(server_api));

        let players = Players::load(format!("{}/{}/players.json", config_path, Self::id(),));
        serenity_data.insert::<Players>(Arc::new(Mutex::new(players)));

        let tp_locations =
            TpLocations::load(format!("{}/{}/tp_locations.json", config_path, Self::id(),));
        serenity_data.insert::<TpLocations>(Arc::new(Mutex::new(tp_locations)));

        tokio::spawn(runner.run(addr.clone()));
        tokio::spawn(client_handler.run(addr.clone()));

        Ok(())
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
    chat_channels: Vec<WebhookConfig>,
}

impl DiscordConfig {
    pub fn is_chat_channel(&self, channel_id: serenity::model::prelude::ChannelId) -> bool {
        // self.chat_channels.iter().map(|c| c.id).contains(&channel_id.0)
        self.chat_channels
            .iter()
            .any(|c| c.id == channel_id.0)
    }
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
struct WebhookConfig {
    id: u64, 
    token: String,
}

struct TypingResults(Vec<Typing>);

impl TypeMapKey for TypingResults {
    type Value = Arc<Mutex<Option<TypingResults>>>;
}
