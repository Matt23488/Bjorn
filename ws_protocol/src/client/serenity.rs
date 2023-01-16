use std::{
    env,
    pin::Pin,
    sync::{Arc, Mutex},
};

use serenity::async_trait;

use crate::{WsChannel, WsClientType};

use super::TaskCanceller;

type BjornSerenityClientFuture =
    Pin<Box<dyn futures_util::Future<Output = Result<BjornSerenityClient, SerenityBjornError>>>>;

pub trait ClientExtension {
    fn with_bjorn(self) -> BjornSerenityClientFuture;
}

impl ClientExtension for serenity::Client {
    fn with_bjorn(self) -> BjornSerenityClientFuture {
        Box::pin(async move {
            let (bot_channel, ws_task, canceller) = with_ws_connection()?;

            with_serenity_config(&self, bot_channel).await?;

            with_ctrlc(&self, canceller)?;

            Ok(BjornSerenityClient::new(self, ws_task))
        })
    }
}

pub struct BjornSerenityClient {
    client: serenity::Client,
    ws_task: tokio::task::JoinHandle<()>,
    data_setters: Option<Vec<Box<dyn FnOnce(&mut serenity::prelude::TypeMap)>>>,
}

impl BjornSerenityClient {
    fn new(client: serenity::Client, ws_task: tokio::task::JoinHandle<()>) -> BjornSerenityClient {
        BjornSerenityClient {
            client,
            ws_task,
            data_setters: Some(vec![]),
        }
    }

    pub async fn start(mut self) -> Result<(), SerenityBjornError> {
        let data_setters = self.data_setters.take().unwrap();
        if data_setters.len() > 0 {
            let mut data = self.client.data.write().await;
            data_setters.into_iter().for_each(|f| f(&mut data));
            drop(data);
        }

        let (_, serenity_result) = tokio::join!(self.ws_task, self.client.start());
        serenity_result?;

        Ok(())
    }

    pub fn with_config<T>(mut self) -> Self
    where
        T: GameConfigTypeMapKey,
    {
        let path = format!("discord_games/{}.json", T::id());
        let config = match std::fs::read_to_string(path) {
            Ok(config) => config,
            Err(_) => {
                println!("Couldn't find config for {}, skipping.", T::id());
                return self;
            }
        };

        let config = match serde_json::from_str::<T::Config>(config.as_str()) {
            Ok(config) => config,
            Err(_) => {
                println!("Counldn't parse {} config, skipping.", T::id());
                return self;
            }
        };

        self.data_setters
            .as_mut()
            .unwrap()
            .push(Box::new(move |data| {
                data.insert::<T>(T::new(config));
            }));

        self
    }
}

type WsConnectionOk = (WsChannel, tokio::task::JoinHandle<()>, TaskCanceller);

fn with_ws_connection() -> Result<WsConnectionOk, SerenityBjornError> {
    let ws_server_addr = env::var("BJORN_WS_CONNECT_ADDRESS")?;
    let (bot_channel, runner, canceller) = super::new(WsClientType::Discord);

    let ws_task = tokio::spawn(runner.run(ws_server_addr));

    Ok((bot_channel, ws_task, canceller))
}

pub enum Role {
    User,
    Admin,
}

impl Role {
    pub fn is_admin(&self) -> bool {
        match self {
            Role::Admin => true,
            _ => false,
        }
    }

    pub fn is_user(&self) -> bool {
        match self {
            Role::User => true,
            _ => false,
        }
    }
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct RoleConfig {
    admin: Vec<u64>,
    user: Vec<u64>,
}

impl RoleConfig {
    async fn has_role_static(
        valid_roles: &Vec<u64>,
        ctx: &serenity::prelude::Context,
        msg: &serenity::model::prelude::Message,
        guild_id: serenity::model::prelude::GuildId,
    ) -> bool {
        for role in valid_roles {
            match msg.author.has_role(ctx, guild_id, *role).await {
                Ok(has_role) if has_role => {
                    return true;
                }
                _ => (),
            }
        }

        false
    }

    pub async fn has_role(
        &self,
        ctx: &serenity::prelude::Context,
        msg: &serenity::model::prelude::Message,
        guild_id: serenity::model::prelude::GuildId,
        role: Role,
    ) -> bool {
        if role.is_admin() && RoleConfig::has_role_static(&self.admin, ctx, msg, guild_id).await {
            true
        } else if role.is_user()
            && RoleConfig::has_role_static(&self.user, ctx, msg, guild_id).await
        {
            true
        } else {
            false
        }
    }
}

#[async_trait]
pub trait GameConfigTypeMapKey: serenity::prelude::TypeMapKey {
    type Config: serde::Serialize + for<'de> serde::Deserialize<'de>;

    fn id() -> &'static str;
    fn new(game_config: Self::Config) -> Self::Value;
    async fn has_necessary_permissions(
        &self,
        ctx: &serenity::prelude::Context,
        msg: &serenity::model::prelude::Message,
        role: Role,
    ) -> bool;
}

async fn with_serenity_config(
    client: &serenity::Client,
    bot_channel: WsChannel,
) -> Result<(), SerenityBjornError> {
    let mut data = client.data.write().await;

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
    CtrlC,
    SerenityRun,
}

impl std::fmt::Display for SerenityBjornError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::InvalidEnvironment => "Bjorn WS environment not configured.",
                Self::CtrlC => "Ctrl+C handler failed.",
                Self::SerenityRun => "Error running Discord client.",
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
        SerenityBjornError::CtrlC
    }
}

impl From<serenity::Error> for SerenityBjornError {
    fn from(_: serenity::Error) -> Self {
        SerenityBjornError::SerenityRun
    }
}

impl From<std::io::Error> for SerenityBjornError {
    fn from(_: std::io::Error) -> Self {
        SerenityBjornError::InvalidEnvironment
    }
}

impl From<serde_json::Error> for SerenityBjornError {
    fn from(_: serde_json::Error) -> Self {
        SerenityBjornError::InvalidEnvironment
    }
}

impl serenity::prelude::TypeMapKey for WsChannel {
    type Value = Mutex<Option<WsChannel>>;
}
