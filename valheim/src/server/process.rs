use std::{
    io::BufRead,
    process::{self, Child, Command, Stdio},
    sync::Arc,
};

pub struct ValheimServerProcess {
    server_dir: String,
    name: String,
    world: String,
    password: String,
    app_id: String,
    valheim: Option<Child>,
    stdout_handler: Option<Arc<dyn Fn(&str, bool) + Send + Sync>>,
    stopped_handler: Option<Arc<dyn Fn() + Send + Sync>>,
    crossplay: Option<bool>,
}

impl ValheimServerProcess {
    pub fn build(dir: &str, name: &str, world: &str, password: &str, app_id: &str) -> Self {
        ValheimServerProcess {
            server_dir: String::from(dir),
            name: String::from(name),
            world: String::from(world),
            password: String::from(password),
            app_id: String::from(app_id),
            valheim: None,
            stdout_handler: None,
            stopped_handler: None,
            crossplay: None,
        }
    }

    fn start_command(&self, crossplay: bool) -> Command {
        let mut start_command = process::Command::new(
            std::path::Path::new(&self.server_dir).join("valheim_server.exe"),
        );

        start_command
            .current_dir(&self.server_dir)
            .args([
                "-nographics",
                "-batchmode",
                "-name",
                &self.name,
                "-port",
                "2456",
                "-world",
                &self.world,
                "-password",
                &self.password,
                "-public",
                "0",
            ])
            .env("SteamAppId", &self.app_id)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped());

        if crossplay {
            start_command.arg("-crossplay");
        }

        start_command
    }

    pub fn start(&mut self, crossplay: bool) -> Result<(), ValheimServerProcessError> {
        if let Some(_) = self.valheim {
            return Err(ValheimServerProcessError::AlreadyStarted);
        }

        let mut child = match self.start_command(crossplay).spawn() {
            Ok(child) => child,
            Err(e) => return Err(ValheimServerProcessError::CouldNotStart(e.to_string())),
        };

        if let Some(handler) = self.stdout_handler.as_ref() {
            let handler = handler.clone();
            let stdout = child.stdout.take().expect("stdout to be piped");
            std::thread::spawn(move || {
                let reader = std::io::BufReader::new(stdout);
                for line in reader.lines() {
                    match line {
                        Ok(line) => handler(line.as_str(), crossplay),
                        Err(_) => break,
                    }
                }
            });
        }

        self.valheim.replace(child);
        self.crossplay.replace(crossplay);

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

        self.valheim
            .take()
            .unwrap()
            .wait()
            .map_err(|e| ValheimServerProcessError::CouldNotStop(e.to_string()))?;

        self.crossplay.take();

        if let Some(stopped_handler) = &self.stopped_handler {
            stopped_handler();
        }

        Ok(())
    }

    pub fn handle_stdout<F>(&mut self, f: F)
    where
        F: Fn(&str, bool) + Send + Sync + 'static,
    {
        self.stdout_handler = Some(Arc::new(f));
    }

    pub fn on_stopped<F>(&mut self, f: F)
    where
        F: Fn() + Send + Sync + 'static,
    {
        self.stopped_handler = Some(Arc::new(f));
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
