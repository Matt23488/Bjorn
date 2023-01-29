use std::{
    env,
    sync::{Arc, Mutex},
};

mod handler;
use discord_config::GameConfig;
use handler::*;

use minecraft::Json;
use serenity::{framework::StandardFramework, prelude::*};
use serenity_ctrlc::Ext;
use ws_protocol::WsTask;

pub struct Bot {
    client: serenity::Client,
}

pub struct UninitializedBot {
    bot_token: String,
    framework: StandardFramework,
}

impl Bot {
    pub fn new() -> Result<UninitializedBot, Error> {
        let prefix = env::var("BJORN_DISCORD_PREFIX")?;
        let bot_token = env::var("BJORN_DISCORD_TOKEN")?;

        let framework = StandardFramework::new().configure(|c| c.prefix(prefix));

        Ok(UninitializedBot {
            bot_token,
            framework,
        })
    }

    pub async fn start(self, addr: &String) -> Result<(), Error> {
        let mut data = self.client.data.write().await;

        data.insert::<minecraft::DiscordConfig>(minecraft::DiscordConfig::new(load_config()));

        let (minecraft_api_components, minecraft_handler_components) =
            minecraft::DiscordConfig::new_ws_clients(&self.client);

        data.insert::<WsClient<minecraft::DiscordConfig>>(Mutex::new(minecraft_api_components.0));

        let players = minecraft::Players::load(format!(
            "discord_games/{}/players.json",
            minecraft::DiscordConfig::id(),
        ));
        data.insert::<minecraft::Players>(Arc::new(Mutex::new(players)));

        let tp_locations = minecraft::TpLocations::load(format!(
            "discord_games/{}/tp_locations.json",
            minecraft::DiscordConfig::id(),
        ));
        data.insert::<minecraft::TpLocations>(Arc::new(Mutex::new(tp_locations)));

        drop(data);

        let cancellers = Arc::new(Mutex::new(Some(vec![
            minecraft_api_components.2,
            minecraft_handler_components.1,
        ])));

        let mut client = self
            .client
            .ctrlc_with(move |dc| {
                let cancellers = cancellers.clone();
                async move {
                    println!("^C");

                    if let Some(cancellers) = cancellers.lock().unwrap().take() {
                        cancellers.into_iter().for_each(|c| c.cancel());
                    }

                    serenity_ctrlc::Disconnector::disconnect_some(dc).await;
                }
            })
            .expect("Ctrl+C Setup failed");

        let serenity_task = client.start();
        let minecraft_ws_task = tokio::spawn(minecraft_api_components.1.run(addr.clone()));
        let minecraft_handler_task = tokio::spawn(minecraft_handler_components.0.run(addr.clone()));

        let (serenity_result, minecraft_ws_result, minecraft_handler_result) =
            tokio::join!(serenity_task, minecraft_ws_task, minecraft_handler_task);
        serenity_result.unwrap_or_default();
        minecraft_ws_result.unwrap_or_default();
        minecraft_handler_result.unwrap_or_default();

        Ok(())
    }
}

fn load_config<T>() -> T
where
    T: GameConfig + serde::Serialize + for<'de> serde::Deserialize<'de>,
{
    serde_json::from_str(
        std::fs::read_to_string(format!("discord_games/{}/config.json", T::id()))
            .expect("Could not load config")
            .as_str(),
    )
    .expect("Could not deserialize config")
}

type WsClient<T> = ws_protocol::WsClient<<T as GameConfig>::Api>;

impl UninitializedBot {
    pub async fn init(mut self) -> Result<Bot, Error> {
        self.framework.group_add(&minecraft::MINECRAFT_GROUP);

        let intents = GatewayIntents::non_privileged() | GatewayIntents::MESSAGE_CONTENT;

        let client = Client::builder(self.bot_token, intents)
            .event_handler(BjornHandler)
            .framework(self.framework)
            .await?;

        Ok(Bot { client })
    }
}

type OnMessage = std::pin::Pin<
    Box<
        dyn Fn(
                &serenity::prelude::Context,
                &serenity::model::prelude::Message,
            ) -> std::pin::Pin<Box<dyn futures_util::Future<Output = ()> + Send>>
            + Send
            + Sync,
    >,
>;

pub struct MessageDispatcher(OnMessage);

impl serenity::prelude::TypeMapKey for MessageDispatcher {
    type Value = Arc<Mutex<Vec<OnMessage>>>;
}

#[derive(Debug)]
pub enum Error {
    Environment,
    SerenityInit,
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Error::Environment => "Environment variables not configured properly.",
                Error::SerenityInit => "Error creating Serenity Discord client.",
            }
        )
    }
}

impl std::error::Error for Error {}

impl From<std::env::VarError> for Error {
    fn from(_: std::env::VarError) -> Self {
        Error::Environment
    }
}

impl From<SerenityError> for Error {
    fn from(_: SerenityError) -> Self {
        Error::SerenityInit
    }
}
