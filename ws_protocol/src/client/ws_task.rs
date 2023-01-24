use async_trait::async_trait;

#[async_trait]
pub trait WsTask {
    async fn run(self, addr: String);
}
