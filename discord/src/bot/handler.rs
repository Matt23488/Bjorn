use discord_config::{use_data, BjornMessageHandler};
use serenity::{async_trait, model::prelude::Message, prelude::*};

pub struct BjornHandler;

#[async_trait]
impl EventHandler for BjornHandler {
    async fn message(&self, ctx: Context, msg: Message) {
        // Ignore Bjorn's own messages
        if msg.author.id == ctx.cache.current_user_id() {
            return;
        }

        use_data!(ctx.data, |minecraft_config: minecraft::DiscordConfig| {
            if minecraft_config.is_chat_channel(msg.channel_id) {
                minecraft::MessageHandler::client_message(&ctx, &msg).await;
            }
        });
    }
}
