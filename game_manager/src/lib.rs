mod manager;
use manager::*;

pub async fn run() {
    let mut manager = GameManager::new();

    manager.register::<minecraft::MinecraftServer>();

    manager.run().await;
}
