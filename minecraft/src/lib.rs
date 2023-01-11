#[cfg(feature = "server")]
mod server;

#[cfg(feature = "server")]
pub use server::MinecraftServer;

#[cfg(feature = "serenity")]
pub mod serenity;

pub enum Message {
    Unknown,
    Start,
    Stop,
    Save,
    Say(String),
    Tp(String),
}
