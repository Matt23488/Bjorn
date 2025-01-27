use std::{
    env,
    sync::{Arc, Mutex},
};

mod handler;
use handler::*;

use serenity::{framework::StandardFramework, prelude::*};
use serenity_ctrlc::Ext;

type SetupFn = Box<
    dyn FnOnce(
        discord_config::DiscordGameSetupData,
        &mut serenity::prelude::TypeMap,
        &mut discord_config::Canceller,
    ) -> Result<(), ()>,
>;

static CONFIG_PATH: &str = "discord_games";

pub struct Bot {
    bot_token: String,
    framework: StandardFramework,
    game_setups: Vec<(&'static str, SetupFn)>,
}

impl Bot {
    pub fn new() -> Result<Bot, Error> {
        let prefix = env::var("BJORN_DISCORD_PREFIX")?;
        let bot_token = env::var("BJORN_DISCORD_TOKEN")?;

        let framework = StandardFramework::new().configure(|c| c.prefix(prefix));

        Ok(Bot {
            bot_token,
            framework,
            game_setups: vec![],
        })
    }

    pub fn with_game<Game>(mut self) -> Self
    where
        Game: discord_config::DiscordGame + 'static,
    {
        self.framework.group_add(Game::command_group());
        self.game_setups.push((Game::id(), Box::new(Game::setup)));

        self
    }

    pub async fn start(self, addr: String) -> Result<(), Error> {
        let intents = GatewayIntents::non_privileged() | GatewayIntents::MESSAGE_CONTENT;

        let client = Client::builder(self.bot_token, intents)
            .event_handler(BjornHandler)
            .framework(self.framework)
            .await?;

        let mut canceller = discord_config::Canceller(vec![]);

        {
            let mut data = client.data.write().await;

            self.game_setups.into_iter().for_each(|(game, setup)| {
                setup(
                    discord_config::DiscordGameSetupData {
                        config_path: String::from(CONFIG_PATH),
                        data: client.data.clone(),
                        cache_and_http: client.cache_and_http.clone(),
                        addr: addr.clone(),
                    },
                    &mut data,
                    &mut canceller,
                )
                .unwrap_or_else(|_| println!("Failed setup for {game}, skipping."));
            });
        }

        let canceller = Arc::new(Mutex::new(Some(canceller)));
        let mut client = client
            .ctrlc_with(move |dc| {
                let canceller = canceller.clone();
                async move {
                    println!("^C");

                    if let Some(canceller) = canceller.lock().unwrap().take() {
                        canceller.cancel();
                    }

                    serenity_ctrlc::Disconnector::disconnect_some(dc).await;
                }
            })
            .expect("Ctrl+C Setup failed");

        client.start().await?;

        Ok(())
    }
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
