use std::sync::{Arc, Mutex};

use serde::{Serialize, Deserialize};

use crate::server::RealmCoords;

#[derive(Serialize, Deserialize)]
struct TpLocation {
    name: String,
    coords: RealmCoords,
}

pub struct TpLocations {
    path: String,
    locations: Vec<TpLocation>,
}

impl TpLocations {
    pub fn load(path: String) -> TpLocations {
        let locations = match std::fs::read_to_string(&path) {
            Ok(json) => {
                serde_json::from_str::<Vec<TpLocation>>(&json).expect("Error parsing tp location list.")
            }
            Err(_) => {
                std::fs::write(&path, "[]").expect("Error accessing tp location config path.");
                vec![]
            }
        };

        TpLocations { path, locations }
    }

    pub fn get_coords(&self, name: &str) -> Option<RealmCoords> {
        Some(self.locations.iter().find(|l| l.name == name)?.coords)
    }

    pub fn save_coords(&mut self, name: String, coords: RealmCoords) -> bool {
        if self
            .locations
            .iter()
            .find(|l| l.name == name)
            .is_some()
        {
            return false;
        }

        self.locations.push(TpLocation {
            name,
            coords,
        });

        std::fs::write(
            &self.path,
            serde_json::to_string(&self.locations).expect("Error serializing tp location list."),
        )
        .expect("Error modifying tp location list.");

        true
    }
}

impl serenity::prelude::TypeMapKey for TpLocations {
    type Value = Arc<Mutex<TpLocations>>;
}
