use std::sync::{Arc, Mutex};

use discord_config::use_data;
use serenity::{
    async_trait, framework::standard::macros::group, http::Typing, model::prelude::Message,
    prelude::*,
};
use ws_protocol::WsTask;

use super::*;

impl TypeMapKey for DiscordConfig {
    type Value = Mutex<Option<DiscordConfig>>;
}

#[group]
#[commands(vstart, vstop, vplayer)] // TODO: Macro to add commands in?
struct Valheim;

pub struct MessageHandler;

#[async_trait]
impl discord_config::BjornMessageHandler for MessageHandler {
    type Handler = client::Handler;

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

        use_data!(data, |config: DiscordConfig| {
            let mut typing_results = vec![];
            for channel in &config.listen_channels {
                let channel = http_and_cache.cache.channel(*channel).unwrap().id();

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
        "valheim"
    }

    fn command_group() -> &'static serenity::framework::standard::CommandGroup {
        &VALHEIM_GROUP
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
}

struct TypingResults(Vec<Typing>);

impl TypeMapKey for TypingResults {
    type Value = Arc<Mutex<Option<TypingResults>>>;
}
