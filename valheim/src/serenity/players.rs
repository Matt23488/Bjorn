use std::sync::{Arc, Mutex};

use serde::{Deserialize, Serialize};

use super::json::Json;

#[derive(Serialize, Deserialize)]
pub struct Player {
    user_id: u64,
    name: String,
}

pub struct Players {
    path: String,
    players: Vec<Player>,
}

impl Json for Players {
    type JsonType = Vec<Player>;

    fn name() -> &'static str {
        "player"
    }

    fn empty_json_str() -> &'static str {
        "[]"
    }

    fn empty_json() -> Self::JsonType {
        vec![]
    }

    fn new(path: String, data: Self::JsonType) -> Self {
        Players {
            path,
            players: data,
        }
    }
}

impl Players {
    pub fn set_player_name(&mut self, user_id: u64, name: String) -> bool {
        if self.players.iter().find(|p| p.name == name).is_some() {
            return false;
        }

        match self.players.iter_mut().find(|p| p.user_id == user_id) {
            Some(player) => player.name = name,
            None => self.players.push(Player { user_id, name }),
        }

        Self::save(&self.path, &self.players);

        true
    }

    pub fn get_user_id(&self, name: &String) -> Option<u64> {
        Some(self.players.iter().find(|p| p.name == *name)?.user_id)
    }

    pub fn get_registered_name(&self, user_id: u64) -> Option<String> {
        self.players
            .iter()
            .find(|p| p.user_id == user_id)
            .map(|p| p.name.clone())
    }
}

impl serenity::prelude::TypeMapKey for Players {
    type Value = Arc<Mutex<Players>>;
}
