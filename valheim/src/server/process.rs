use std::{
    io::BufRead,
    process::{self, Child, ChildStdin, Command, Stdio},
    sync::Arc,
};

pub struct ValheimServerProcess {
    start_command: Command,
    valheim: Option<Child>,
    stdin: Option<ChildStdin>, // TODO: Probably don't need to pipe stdin since Valheim doesn't use it
    stdout_handler: Option<Arc<dyn Fn(&str) + Send + Sync>>,
}

impl ValheimServerProcess {
    pub fn build(dir: &str, name: &str, world: &str, password: &str, app_id: &str) -> Self {
        let mut start_command =
            process::Command::new(std::path::Path::new(dir).join("valheim_server.exe"));

        start_command
            .current_dir(dir)
            .args([
                "-nographics",
                "-batchmode",
                "-name",
                name,
                "-port",
                "2456",
                "-world",
                world,
                "-password",
                password,
                // "-crossplay", // TODO: Probably make this configurable
                "-public",
                "0",
            ])
            .env("SteamAppId", app_id)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped());

        ValheimServerProcess {
            start_command,
            valheim: None,
            stdin: None,
            stdout_handler: None,
        }
    }

    pub fn start(&mut self) -> Result<(), ValheimServerProcessError> {
        if let Some(_) = self.valheim {
            return Err(ValheimServerProcessError::AlreadyStarted);
        }

        let mut child = match self.start_command.spawn() {
            Ok(child) => child,
            Err(e) => return Err(ValheimServerProcessError::CouldNotStart(e.to_string())),
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

        self.valheim.replace(child);

        Ok(())
    }

    pub fn stop(&mut self) -> Result<(), ValheimServerProcessError> {
        if let None = self.valheim {
            return Err(ValheimServerProcessError::NotRunning);
        }

        process::Command::new("taskkill")
            .args(["/IM", "valheim_server.exe"])
            .spawn()
            .map_err(|e| ValheimServerProcessError::CouldNotStop(e.to_string()))?;

        drop(self.stdin.take());

        self.valheim
            .take()
            .unwrap()
            .wait()
            .map_err(|e| ValheimServerProcessError::CouldNotStop(e.to_string()))?;

        Ok(())
    }

    pub fn handle_stdout<F>(&mut self, f: F)
    where
        F: Fn(&str) + Send + Sync + 'static,
    {
        self.stdout_handler = Some(Arc::new(f));
    }

    pub fn is_running(&self) -> bool {
        self.valheim.is_some()
    }
}

#[derive(Debug)]
pub enum ValheimServerProcessError {
    AlreadyStarted,
    NotRunning,
    CouldNotStart(String),
    CouldNotStop(String),
}

impl std::fmt::Display for ValheimServerProcessError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                ValheimServerProcessError::AlreadyStarted =>
                    "Valheim server already started.".into(),
                ValheimServerProcessError::NotRunning => "Valheim server not running.".into(),
                ValheimServerProcessError::CouldNotStart(err) =>
                    format!("Valheim server couldn't start: {err}"),
                ValheimServerProcessError::CouldNotStop(err) =>
                    format!("Valheim server couldn't stop: {err}"),
            }
        )
    }
}

impl std::error::Error for ValheimServerProcessError {}
