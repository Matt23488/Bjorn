use std::{
    io::{BufRead, Write},
    process::{self, Child, ChildStdin, Command, Stdio},
    sync::Arc,
};

pub struct MinecraftServerProcess {
    start_command: Command,
    minecraft: Option<Child>,
    stdin: Option<ChildStdin>,
    stdout_handler: Option<Arc<dyn Fn(&str) + Send + Sync>>,
}

impl MinecraftServerProcess {
    pub fn build(dir: &str) -> Self {
        let mut start_command = process::Command::new("java");

        start_command
            .current_dir(dir)
            .args(["-Xmx1024M", "-Xms1024M", "-jar", "server.jar", "nogui"])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped());

        MinecraftServerProcess {
            start_command,
            minecraft: None,
            stdin: None,
            stdout_handler: None,
        }
    }

    pub fn start(&mut self) -> Result<(), MinecraftServerProcessError> {
        if let Some(_) = self.minecraft {
            return Err(MinecraftServerProcessError::AlreadyStarted);
        }

        let mut child = match self.start_command.spawn() {
            Ok(child) => child,
            Err(e) => return Err(MinecraftServerProcessError::CouldNotStart(e.to_string())),
        };

        self.stdin
            .replace(child.stdin.take().expect("stdin to be piped"));

        if let Some(handler) = self.stdout_handler.as_ref() {
            let handler = handler.clone();
            let stdout = child.stdout.take().expect("stdout to be piped");
            std::thread::spawn(move || {
                let reader = std::io::BufReader::new(stdout);
                for line in reader.lines() {
                    match line {
                        Ok(line) => handler(line.as_str()),
                        Err(_) => break,
                    }
                }
            });
        }

        self.minecraft.replace(child);

        Ok(())
    }

    pub fn stop(&mut self) -> Result<(), MinecraftServerProcessError> {
        self.send_to_stdin(b"stop\n")?;

        drop(self.stdin.take());

        match self.minecraft.take().unwrap().wait() {
            Ok(_) => Ok(()),
            Err(e) => Err(MinecraftServerProcessError::CouldNotStop(e.to_string())),
        }
    }

    pub fn handle_stdout<F>(&mut self, f: F)
    where
        F: Fn(&str) + Send + Sync + 'static,
    {
        self.stdout_handler = Some(Arc::new(f));
    }

    pub fn save(&mut self) -> Result<(), MinecraftServerProcessError> {
        self.send_to_stdin(b"save-all\n")
    }

    pub fn chat(&mut self, user: &str, message: &str) -> Result<(), MinecraftServerProcessError> {
        self.send_to_stdin(format!("say (Discord) {user}: {message}\n").as_bytes())
    }

    pub fn tp(&mut self, player: &str, target: &str) -> Result<(), MinecraftServerProcessError> {
        self.send_to_stdin(format!("tp {player} {target}\n").as_bytes())
    }

    fn send_to_stdin(&mut self, bytes: &[u8]) -> Result<(), MinecraftServerProcessError> {
        if let None = self.minecraft {
            return Err(MinecraftServerProcessError::NotRunning);
        }

        self.stdin
            .as_mut()
            .expect("stdin to be Some")
            .write_all(bytes)
            .expect("write_all to succeed");

        Ok(())
    }
}

#[derive(Debug)]
pub enum MinecraftServerProcessError {
    AlreadyStarted,
    NotRunning,
    CouldNotStart(String),
    CouldNotStop(String),
}

impl std::fmt::Display for MinecraftServerProcessError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                MinecraftServerProcessError::AlreadyStarted =>
                    "Minecraft server already started.".into(),
                MinecraftServerProcessError::NotRunning => "Minecraft server not running.".into(),
                MinecraftServerProcessError::CouldNotStart(err) =>
                    format!("Minecraft server couldn't start: {err}"),
                MinecraftServerProcessError::CouldNotStop(err) =>
                    format!("Minecraft server couldn't stop: {err}"),
            }
        )
    }
}

impl std::error::Error for MinecraftServerProcessError {}
