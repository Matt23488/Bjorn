#[macro_use]
extern crate rocket;

use std::thread;

use websocket::ClientBuilder;

mod cors;
use cors::CORS;

#[get("/")]
fn index() -> &'static str {
    "poopy pants"
}

#[rocket::main]
async fn main() -> Result<(), rocket::Error> {
    let ws_thread = thread::spawn(|| {
        let client = ClientBuilder::new("ws://127.0.0.1:42069")
            .unwrap()
            .connect_insecure()
            .unwrap();

        let (mut receiver, _) = client.split().unwrap();

        for message in receiver.incoming_messages() {
            match message {
                Ok(message) => println!("Received: {message:?}"),
                Err(_) => (),
            }
        }
    });

    let figment = rocket::Config::figment().merge(("port", 64209));

    let _rocket = rocket::custom(figment)
        .mount("/", routes![index])
        .attach(CORS)
        .launch()
        .await?;

    ws_thread.join().unwrap();

    Ok(())
}
