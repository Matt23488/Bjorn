use std::sync::{Arc, Mutex};

#[derive(serde::Serialize, serde::Deserialize)]
struct Player {
    user_id: u64,
    name: String,
}

pub struct Players {
    path: String,
    players: Vec<Player>,
}

impl Players {
    pub fn load(path: String) -> Players {
        let players = match std::fs::read_to_string(&path) {
            Ok(json) => {
                serde_json::from_str::<Vec<Player>>(&json).expect("Error parsing player list.")
            }
            Err(_) => {
                std::fs::write(&path, "[]").expect("Error accessing player config path.");
                vec![]
            }
        };

        Players { path, players }
    }

    pub fn set_player_name(&mut self, user_id: u64, name: String) -> bool {
        if self
            .players
            .iter()
            .find(|p| p.name == name)
            .is_some()
        {
            return false;
        }

        match self.players.iter_mut().find(|p| p.user_id == user_id) {
            Some(player) => player.name = name,
            None => self.players.push(Player {
                user_id,
                name,
            }),
        }

        std::fs::write(
            &self.path,
            serde_json::to_string(&self.players).expect("Error serializing player list."),
        )
        .expect("Error modifying player list.");

        true
    }

    pub fn get_user_id(&self, name: &String) -> Option<u64> {
        Some(
            self.players
                .iter()
                .find(|p| p.name == *name)?
                .user_id,
        )
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
