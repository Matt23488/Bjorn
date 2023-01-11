use serenity::{prelude::*, framework::standard::CommandResult};
use serenity::framework::standard::macros::command;
use serenity::model::channel::Message;

#[command]
pub async fn test(ctx: &Context, msg: &Message) -> CommandResult {
    msg.reply(ctx, "idk").await.unwrap();
    
    Ok(())
}