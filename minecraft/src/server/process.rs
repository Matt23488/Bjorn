use std::{
    io::{BufRead, Write}, path::{Path, PathBuf}, process::{self, Child, ChildStdin, Command, Stdio}, sync::Arc
};

pub struct MinecraftServerProcess {
    start_command: Command,
    server_path: PathBuf,
    backup_path: Option<String>,
    minecraft: Option<Child>,
    stdin: Option<ChildStdin>,
    stdout_handler: Option<Arc<dyn Fn(&str) + Send + Sync>>,
}

impl MinecraftServerProcess {
    pub fn build(dir: &str, server_jar: &str, max_memory: &str, backup_path: Option<String>) -> Self {
        let mut start_command = process::Command::new("java");

        start_command
            .current_dir(dir)
            .args([&format!("-Xmx{}", max_memory), "-jar", server_jar, "nogui"])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped());

        MinecraftServerProcess {
            start_command,
            server_path: Path::new(dir).to_path_buf(),
            backup_path,
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

    pub fn command(&mut self, command_text: &str) -> Result<(), MinecraftServerProcessError> {
        self.send_to_stdin(command_text.as_bytes())
    }

    pub fn tp_loc(
        &mut self,
        player: &str,
        realm: &str,
        x: f64,
        y: f64,
        z: f64,
    ) -> Result<(), MinecraftServerProcessError> {
        self.send_to_stdin(
            format!("execute as {player} in {realm} run teleport {x} {y} {z}\n").as_bytes(),
        )
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

    pub fn is_running(&self) -> bool {
        self.minecraft.is_some()
    }

    pub fn backup_server(&self) -> Result<WorldBackupResult, MinecraftServerProcessError> {
        let (backup_path, dir_name) = match &self.backup_path {
            Some(backup_path) => {
                let dir_name = chrono::Local::now().format("%Y_%m%d_%H%M%S").to_string();
                let backup_path = std::path::Path::new(backup_path).join(&dir_name);
                
                Ok((backup_path, dir_name))
            },
            None => Err(MinecraftServerProcessError::BackupPathNotConfigured),
        }?;
        
        let world_size = super::fs::copy_dir(self.server_path.as_path(), backup_path.as_path())?;

        Ok(WorldBackupResult {
            dir_name,
            size: world_size,
        })
    }
}

#[derive(Debug)]
pub enum MinecraftServerProcessError {
    AlreadyStarted,
    NotRunning,
    CouldNotStart(String),
    CouldNotStop(String),
    BackupPathNotConfigured,
    BackupFailed(std::io::Error),
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
                MinecraftServerProcessError::BackupPathNotConfigured =>
                    "Backup path not configured.".into(),
                MinecraftServerProcessError::BackupFailed(err) =>
                    format!("Backup failed: {err}"),
            }
        )
    }
}

impl std::error::Error for MinecraftServerProcessError {}

impl From<std::io::Error> for MinecraftServerProcessError {
    fn from(value: std::io::Error) -> Self {
        Self::BackupFailed(value)
    }
}

pub struct WorldBackupResult {
    pub dir_name: String,
    pub size: u64,
}
