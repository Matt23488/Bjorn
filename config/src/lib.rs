use std::fs;
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct Config {
    prefix: String,
}

impl Config {
    pub fn load_specific<T>(filename: &str) -> Option<T>
    where
        for<'de> T: Serialize + Deserialize<'de>,
    {
        let config = match fs::read_to_string(filename) {
            Ok(config) => config,
            Err(_) => return None,
        };

        match serde_json::from_str(config.as_str()) {
            Ok(config) => Some(config),
            Err(_) => None,
        }
    }

    pub fn prefix(&self) -> &String {
        &self.prefix
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Secrets {
    bot_token: String,
}

impl Secrets {
    fn load(filename: &str) -> Option<Secrets> {
        let secrets = match fs::read_to_string(filename) {
            Ok(secrets) => secrets,
            Err(_) => return None,
        };

        match serde_json::from_str(secrets.as_str()) {
            Ok(secrets) => Some(secrets),
            Err(_) => None,
        }
    }

    pub fn bot_token(&self) -> &String {
        &self.bot_token
    }
}

pub struct Environment;

impl Environment {
    pub fn load<TConfig>(config_file_path: &str, secrets_file_path: &str) -> Option<(TConfig, Secrets)>
    where
        for<'de> TConfig: Serialize + Deserialize<'de>,
    {
        match (Config::load_specific(config_file_path), Secrets::load(secrets_file_path)) {
            (Some(config), Some(secrets)) => Some((config, secrets)),
            _ => None,
        }
    }
}
