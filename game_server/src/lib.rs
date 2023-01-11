pub trait ServerProcess {
    fn build(dir: String) -> Result<Self, String>
    where
        Self: Sized;
    fn start(&mut self) -> Result<(), String>;
    fn stop(&mut self) -> Result<(), String>;
}

#[cfg(feature = "serenity")]
mod serenity;

#[cfg(feature = "serenity")]
pub use crate::serenity::*;
