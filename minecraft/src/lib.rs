pub mod server;

// TODO: I need to make this not dependent on serenity.
// #[cfg(feature = "serenity")]
pub mod client;

// #[cfg(feature = "serenity")]
mod serenity;

// #[cfg(feature = "serenity")]
pub use crate::serenity::*;
