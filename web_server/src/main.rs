#[macro_use]
extern crate rocket;

// use std::thread;


mod cors;
use cors::CORS;
use ws_protocol::{BjornWsClient, BjornWsClientType};

#[get("/")]
fn index() -> &'static str {
    "poopy pants"
}

// TODO: I need to nix the server alltogether and just use the ws_server as the backend for the web client
#[rocket::main]
async fn main() -> Result<(), rocket::Error> {
    let _ws_client = BjornWsClient::new(BjornWsClientType::WebServer);

    let _rocket = rocket::build()
        .mount("/", routes![index])
        .attach(CORS)
        .launch()
        .await?;

    Ok(())
}
