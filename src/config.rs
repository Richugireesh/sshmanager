use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Server {
    pub name: String,
    pub user: String,
    pub host: String,
    pub port: u16,
}

pub struct Config {
    pub servers: Vec<Server>,
}

impl Config {
    pub fn load() -> Result<Self, Box<dyn std::error::Error>> {
        let config_path = get_config_path()?;
        if !config_path.exists() {
            return Ok(Config { servers: vec![] });
        }

        let content = fs::read_to_string(config_path)?;
        let servers: Vec<Server> = serde_json::from_str(&content)?;
        Ok(Config { servers })
    }

    pub fn save(&self) -> Result<(), Box<dyn std::error::Error>> {
        let config_path = get_config_path()?;
        if let Some(parent) = config_path.parent() {
            fs::create_dir_all(parent)?;
        }

        let content = serde_json::to_string_pretty(&self.servers)?;
        fs::write(config_path, content)?;
        Ok(())
    }

    pub fn add_server(&mut self, server: Server) {
        self.servers.push(server);
    }

    pub fn remove_server(&mut self, index: usize) {
        if index < self.servers.len() {
            self.servers.remove(index);
        }
    }
}

fn get_config_path() -> Result<PathBuf, Box<dyn std::error::Error>> {
    let mut path = dirs::config_dir().ok_or("Could not find config directory")?;
    path.push("ssh-manager");
    path.push("servers.json");
    Ok(path)
}
