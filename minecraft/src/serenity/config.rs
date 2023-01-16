use serenity::{async_trait, model::prelude::Message, prelude::*};
use ws_protocol::client::GameConfigTypeMapKey;

impl TypeMapKey for DiscordConfig {
    type Value = std::sync::Mutex<Option<DiscordConfig>>;
}

#[async_trait]
impl GameConfigTypeMapKey for DiscordConfig {
    type Config = DiscordConfig;

    fn id() -> &'static str {
        "minecraft"
    }

    fn new(game_config: Self::Config) -> Self::Value {
        std::sync::Mutex::new(Some(game_config))
    }

    async fn has_necessary_permissions(
        &self,
        ctx: &Context,
        msg: &Message,
        role: ws_protocol::client::Role,
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
    roles: ws_protocol::client::RoleConfig,
    listen_channels: Vec<u64>,
    chat_channels: Vec<u64>,
}
