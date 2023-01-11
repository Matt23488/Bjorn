use std::{
    io::Write,
    process::{self, Child, ChildStdin, ChildStdout, Command, Stdio},
};

use game_server::Server;

pub struct MinecraftServer {
    start_command: Command,
    minecraft: Option<Child>,
    stdin: Option<ChildStdin>,
    stdout: Option<ChildStdout>,
}

impl MinecraftServer {
    pub fn save(&mut self) -> Result<(), String> {
        self.send_to_stdin(b"save-all\n")
    }

    pub fn say(&mut self, message: String) -> Result<(), String> {
        self.send_to_stdin(format!("say {message}\n").as_bytes())
    }

    pub fn tp(&mut self, args: String) -> Result<(), String> {
        self.send_to_stdin(format!("tp {args}\n").as_bytes())
    }

    fn send_to_stdin(&mut self, bytes: &[u8]) -> Result<(), String> {
        if let None = self.minecraft {
            return Err("Server not started".into());
        }

        self.stdin
            .as_mut()
            .expect("stdin to be Some")
            .write_all(bytes)
            .expect("write_all to succeed");

        Ok(())
    }
}

impl Server for MinecraftServer {
    fn build() -> Result<Self, String> {
        let mut start_command = process::Command::new(
            r#"C:\Program Files\Eclipse Adoptium\jre-19.0.1.10-hotspot\bin\java.exe"#,
        );

        start_command
            .current_dir(r#"C:\Minecraft"#)
            .args([
                "-Xmx1024M",
                "-Xms1024M",
                "-jar",
                r#"C:\Minecraft\server.jar"#,
                "nogui",
            ])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped());

        Ok(MinecraftServer {
            start_command,
            minecraft: None,
            stdin: None,
            stdout: None,
        })
    }

    fn start(&mut self) -> Result<(), String> {
        if let Some(_) = self.minecraft {
            return Err("Server already started".into());
        }

        let mut child = match self.start_command.spawn() {
            Ok(child) => child,
            Err(e) => return Err(e.to_string()),
        };

        self.stdin
            .replace(child.stdin.take().expect("stdin to be piped"));
        self.stdout
            .replace(child.stdout.take().expect("stdout to be piped"));
        self.minecraft.replace(child);

        Ok(())
    }

    fn stop(&mut self) -> Result<(), String> {
        self.send_to_stdin(b"stop\n")?;

        drop(self.stdin.take());
        drop(self.stdout.take());

        match self.minecraft.take().unwrap().wait() {
            Ok(_) => Ok(()),
            Err(e) => Err(e.to_string()),
        }
    }
}
