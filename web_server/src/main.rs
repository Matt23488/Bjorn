#[macro_use]
extern crate rocket;

// use std::thread;


mod cors;
use cors::CORS;
use ws_protocol::BjornWsClient;

#[get("/")]
fn index() -> &'static str {
    "poopy pants"
}

#[rocket::main]
async fn main() -> Result<(), rocket::Error> {
    let _ws_client = BjornWsClient::new("web_server");

    let figment = rocket::Config::figment().merge(("port", 64209));

    let _rocket = rocket::custom(figment)
        .mount("/", routes![index])
        .attach(CORS)
        .launch()
        .await?;

    Ok(())
}
