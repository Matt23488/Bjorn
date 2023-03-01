use std::sync::{Arc, Mutex};

use serde::{Deserialize, Serialize};

use super::json::Json;

#[derive(Clone, Serialize, Deserialize)]
pub struct AttackMessage {
    pub name: String,
    pub message: String,
    pub image: String,
}

#[derive(Clone)]
pub struct AttackMessages {
    _path: String,
    messages: Vec<AttackMessage>,
}

impl Json for AttackMessages {
    type JsonType = Vec<AttackMessage>;

    fn name() -> &'static str {
        "attack_message"
    }

    fn empty_json_str() -> &'static str {
        "[]"
    }

    fn empty_json() -> Self::JsonType {
        vec![]
    }

    fn new(path: String, data: Self::JsonType) -> Self {
        AttackMessages {
            _path: path,
            messages: data,
        }
    }
}

impl AttackMessages {
    pub fn get_attack_message<'m>(&'m self, id: &str) -> Option<&'m AttackMessage> {
        self.messages.iter().find(|m| m.name == id)
    }
}

impl serenity::prelude::TypeMapKey for AttackMessages {
    type Value = Arc<Mutex<AttackMessages>>;
}
