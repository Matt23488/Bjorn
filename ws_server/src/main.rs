

use std::{process::{self, Stdio}, io::Write, sync::{Arc, Mutex}};

use ws_protocol::BjornWsServer;

fn main() {
    let server = Arc::new(Mutex::new(BjornWsServer::new(|text| {
        match text.as_str() {
            "minecraft start" => {
                let mut server = match process::Command::new(r#"C:\Program Files\Eclipse Adoptium\jre-19.0.1.10-hotspot\bin\java.exe"#)
                    .current_dir(r#"C:\Minecraft"#)
                    .args(["-Xmx1024M", "-Xms1024M", "-jar", r#"C:\Minecraft\server.jar"#])//, "nogui"])
                    .stdin(Stdio::piped())
                    .stdout(Stdio::piped())
                    .spawn()
                {
                    Ok(server) => server,
                    Err(e) => {
                        println!("Couldn't start server: {e}");
                        return;
                    },
                };

                std::thread::spawn(move || {
                    std::thread::sleep(std::time::Duration::from_secs(30));
                    println!("killing Minecraft server");

                    let mut stdin = server.stdin.take().expect("stdin to be Some");
                    stdin.write_all(b"stop\n").expect("write_all to succeed");

                    drop(stdin);

                    server.wait().expect("wait to succeed");
                    println!("Minecraft server stopped");
                });
            }
            _ => ()
        }
    })));

    {
        let server = server.clone();
        ctrlc::set_handler(move || {
            println!("Ctrl+C detected");
            server
                .lock()
                .expect("lock to be valid")
                .stop();
        }).expect("Error setting Ctrl+C handler");
    }

    server
        .lock()
        .expect("lock to be valid")
        .wait();
}
