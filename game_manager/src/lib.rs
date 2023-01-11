mod ws;

mod server;
use server::*;

pub fn run() {
    let mut manager = GameManager::new();

    manager.register::<MinecraftServer>();

    manager.wait_for_cancel();
}
