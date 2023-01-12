mod manager;
use manager::*;

pub fn run() {
    let mut manager = GameManager::new();

    manager.register::<minecraft::MinecraftServer>();

    manager.wait_for_cancel();
}
