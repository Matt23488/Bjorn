use std::{env, error::Error};

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let addr = env::var("BJORN_WS_LISTEN_ADDRESS")?;

    let (runner, canceller) = ws_protocol::server::new();

    let mut canceller = Some(canceller);
    ctrlc::set_handler(move || {
        println!("^C");

        if let Some(canceller) = canceller.take() {
            canceller.cancel();
        }
    })
    .expect("Ctrl+C");

    runner.run(addr).await;

    Ok(())
}
