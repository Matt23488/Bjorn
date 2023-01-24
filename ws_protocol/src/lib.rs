use serde::{Deserialize, Serialize};

#[derive(Debug, Hash, Eq, PartialEq, Serialize, Deserialize, Clone)]
pub enum ApiSpecifier {
    Emits(String),
    Handles(String),
}

#[derive(Debug, Serialize, Deserialize)]
pub enum Handshake {
    ServerIdentification,
    ClientIdentification(ApiSpecifier),
    Web,
}

impl TryFrom<tungstenite::Message> for Handshake {
    type Error = String;

    fn try_from(msg: tungstenite::Message) -> Result<Self, Self::Error> {
        match msg {
            tungstenite::Message::Text(json) => match serde_json::from_str(json.as_str()) {
                Ok(handshake_response) => Ok(handshake_response),
                Err(e) => Err(format!(
                    "Error deserializing message: {e}\nMessage:\n{json}"
                )),
            },
            msg => Err(format!("Cannot parse Handshake response from {msg}")),
        }
    }
}

impl From<Handshake> for tungstenite::Message {
    fn from(handshake: Handshake) -> Self {
        let json = serde_json::to_string(&handshake).unwrap();
        tungstenite::Message::Text(json)
    }
}

mod message;

#[cfg(feature = "client")]
mod client;

#[cfg(feature = "client")]
pub use client::*;

#[cfg(feature = "server")]
mod server;

#[cfg(feature = "server")]
pub use server::*;

#[cfg(feature = "serenity")]
pub mod serenity;
