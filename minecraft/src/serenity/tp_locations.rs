use std::sync::{Arc, Mutex};

use serde::{Serialize, Deserialize};

use crate::{server::RealmCoords, Json};

#[derive(Serialize, Deserialize)]
pub struct TpLocation {
    name: String,
    coords: RealmCoords,
}

pub struct TpLocations {
    path: String,
    locations: Vec<TpLocation>,
}

impl Json for TpLocations {
    type JsonType = Vec<TpLocation>;

    fn name() -> &'static str {
        "tp location"
    }

    fn empty_json_str() -> &'static str {
        "[]"
    }

    fn empty_json() -> Self::JsonType {
        vec![]
    }

    fn new(path: String, data: Self::JsonType) -> Self {
        TpLocations {
            path,
            locations: data,
        }
    }
}

impl TpLocations {
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

        Self::save(&self.path, &self.locations);

        true
    }
}

impl serenity::prelude::TypeMapKey for TpLocations {
    type Value = Arc<Mutex<TpLocations>>;
}
