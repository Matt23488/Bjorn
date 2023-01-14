pub trait ServerProcess {
    fn build(dir: &str) -> Result<Self, String>
    where
        Self: Sized;
    fn start(&mut self) -> Result<(), String>;
    fn stop(&mut self) -> Result<(), String>;
}

#[cfg(feature = "ws_protocol")]
mod ws_protocol;

#[cfg(feature = "ws_protocol")]
pub use crate::ws_protocol::*;
