use std::{
    env,
    pin::Pin,
    sync::{Arc, Mutex},
};

use crate::{WsChannel, WsClientType};

use super::TaskCanceller;

type BjornSerenityClientFuture =
    Pin<Box<dyn futures_util::Future<Output = Result<(), SerenityBjornError>>>>;
pub trait ClientExtension {
    fn start_with_bjorn(self) -> BjornSerenityClientFuture;
}

impl ClientExtension for serenity::Client {
    fn start_with_bjorn(mut self) -> BjornSerenityClientFuture {
        Box::pin(async move {
            let (bot_channel, ws_task, canceller) = with_ws_connection()?;

            with_serenity_config(&self, bot_channel).await?;

            with_ctrlc(&self, canceller)?;

            let (_, serenity_result) = tokio::join!(ws_task, self.start());
            serenity_result.unwrap();

            Ok(())
        })
    }
}

type WsConnectionOk = (WsChannel, tokio::task::JoinHandle<()>, TaskCanceller);

fn with_ws_connection() -> Result<WsConnectionOk, SerenityBjornError> {
    let ws_server_addr = env::var("BJORN_WS_CONNECT_ADDRESS")?;
    let (bot_channel, runner, canceller) = super::new(WsClientType::Discord);

    let ws_task = tokio::spawn(runner.run(ws_server_addr));

    Ok((bot_channel, ws_task, canceller))
}

async fn with_serenity_config(
    client: &serenity::Client,
    bot_channel: WsChannel,
) -> Result<(), SerenityBjornError> {
    let mut data = client.data.write().await;

    let channel = match env::var("BJORN_MINECRAFT_DISCORD_COMMAND_CHANNEL") {
        Err(_) => None,
        Ok(channel) => channel.parse::<u64>().ok(),
    };

    let user = match env::var("BJORN_MINECRAFT_DISCORD_ADMIN") {
        Err(_) => None,
        Ok(admin) => admin.parse::<u64>().ok(),
    };

    let config = match (channel, user) {
        (Some(channel_id), Some(user_id)) => DiscordConfig {
            channel_id,
            user_id,
        },
        _ => return Err(SerenityBjornError::InvalidEnvironment),
    };

    data.insert::<DiscordConfig>(Mutex::new(Some(config)));
    data.insert::<WsChannel>(Mutex::new(Some(bot_channel)));
    drop(data);

    Ok(())
}

fn with_ctrlc(
    client: &serenity::Client,
    canceller: TaskCanceller,
) -> Result<(), SerenityBjornError> {
    let canceller = Arc::new(Mutex::new(Some(canceller)));

    serenity_ctrlc::ctrlc_with(client, move |dc| {
        let canceller = canceller.clone();
        async move {
            println!("Ctrl+C detected");

            println!("Closing WebSocket client");
            canceller.lock().unwrap().take().unwrap().cancel();

            println!("Disconnecting Discord bot");
            serenity_ctrlc::Disconnector::disconnect_some(dc).await;
        }
    })?;

    Ok(())
}

#[derive(Debug)]
pub enum SerenityBjornError {
    InvalidEnvironment,
    CtrlCError,
}

impl std::fmt::Display for SerenityBjornError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::InvalidEnvironment => "Bjorn WS environment not configured.",
                Self::CtrlCError => "Ctrl+C handler failed.",
            }
        )
    }
}

impl std::error::Error for SerenityBjornError {}

impl From<env::VarError> for SerenityBjornError {
    fn from(_: env::VarError) -> Self {
        SerenityBjornError::InvalidEnvironment
    }
}

impl From<serenity_ctrlc::Error> for SerenityBjornError {
    fn from(_: serenity_ctrlc::Error) -> Self {
        SerenityBjornError::CtrlCError
    }
}

pub struct DiscordConfig {
    channel_id: u64,
    user_id: u64,
}

impl DiscordConfig {
    pub fn channel_id(&self) -> u64 {
        self.channel_id
    }

    pub fn user_id(&self) -> u64 {
        self.user_id
    }
}

impl serenity::prelude::TypeMapKey for DiscordConfig {
    type Value = Mutex<Option<DiscordConfig>>;
}

impl serenity::prelude::TypeMapKey for WsChannel {
    type Value = Mutex<Option<WsChannel>>;
}
