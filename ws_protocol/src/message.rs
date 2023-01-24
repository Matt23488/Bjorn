use serde::{Deserialize, Serialize};

use crate::ApiSpecifier;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub target: String,
    pub content: String,
}

impl Message {
    pub fn target_api_specifier(&self) -> ApiSpecifier {
        ApiSpecifier::Handles(self.target.clone())
    }

    pub fn source_api_specifier(&self) -> ApiSpecifier {
        ApiSpecifier::Emits(self.target.clone())
    }
}

impl TryFrom<tungstenite::Message> for Message {
    type Error = String;

    fn try_from(msg: tungstenite::Message) -> Result<Self, Self::Error> {
        match msg {
            tungstenite::Message::Text(json) => match serde_json::from_str(json.as_str()) {
                Ok(msg) => Ok(msg),
                Err(e) => Err(format!("Error parsing message: {e}\nMessage:\n{json}")),
            },
            msg => Err(format!("Cannot parse message from {msg}")),
        }
    }
}

impl From<Message> for tungstenite::Message {
    fn from(msg: Message) -> Self {
        tungstenite::Message::Text(serde_json::to_string(&msg).unwrap())
    }
}
