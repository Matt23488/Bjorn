use std::env;

use game_server::{Dispatcher, DispatchResult};
use game_server_macro::bjorn_command;

#[bjorn_command("Starting Minecraft server...")]
pub fn start(ws: &Dispatcher) -> DispatchResult {
    ws.dispatch("minecraft start".into())
}

#[bjorn_command("Stopping Minecraft server...")]
pub fn stop(ws: &Dispatcher) -> DispatchResult {
    ws.dispatch("minecraft stop".into())
}

#[bjorn_command("Saving Minecraft server...")]
pub fn save(ws: &Dispatcher) -> DispatchResult {
    ws.dispatch("minecraft save".into())
}

#[bjorn_command("Sending chat to Minecraft server...")]
pub fn say(ws: &Dispatcher, args: &str) -> DispatchResult {
    ws.dispatch(format!("minecraft say {args}"))
}

#[bjorn_command("Sending tp command Minecraft server...")]
pub fn tp(ws: &Dispatcher, args: &str) -> DispatchResult {
    ws.dispatch(format!("minecraft tp {args}"))
}
