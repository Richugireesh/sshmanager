use aes_gcm::{
    aead::{Aead, KeyInit, OsRng},
    Aes256Gcm, Nonce,
};
use base64::{engine::general_purpose, Engine as _};
use pbkdf2::pbkdf2;
use rand::RngCore;
use serde::{Deserialize, Serialize};
use hmac::Hmac;
use sha2::Sha256;
use std::fs;
use std::io::BufReader;
use std::path::PathBuf;
use rpassword;
use ssh2_config::SshConfig;

const SALT_LEN: usize = 16;
const NONCE_LEN: usize = 12;
const ITERATIONS: u32 = 100_000;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum AuthType {
    Password(String),
    Key(String), // Path to key
    Agent,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Server {
    pub name: String,
    pub user: String,
    pub host: String,
    pub port: u16,
    pub auth_type: AuthType,
    #[serde(default = "default_group")]
    pub group: String,
}

fn default_group() -> String {
    "General".to_string()
}

#[derive(Deserialize)]
struct LegacyServer {
    name: String,
    user: String,
    host: String,
    port: u16,
}

#[derive(Serialize, Deserialize)]
struct EncryptedConfig {
    salt: String,
    nonce: String,
    ciphertext: String,
}

pub struct Config {
    pub servers: Vec<Server>,
    master_password: Option<String>,
}

impl Config {
    pub fn new() -> Self {
        Config {
            servers: vec![],
            master_password: None,
        }
    }

    pub fn load() -> Result<Self, Box<dyn std::error::Error>> {
        let config_path = get_config_path()?;
        
        if !config_path.exists() {
            return Ok(Config::new());
        }

        let content = fs::read_to_string(&config_path)?;
        
        // 1. Unencrypted New Format
        if let Ok(mut servers) = serde_json::from_str::<Vec<Server>>(&content) {
             // Ensure group is set (handled by serde default but explicit check doesn't hurt if we were manually parsing)
             return Ok(Config { servers, master_password: None });
        }

        // 2. Legacy Format
        if let Ok(legacy_servers) = serde_json::from_str::<Vec<LegacyServer>>(&content) {
            println!("ℹ️  Legacy configuration detected. Migrating...");
            let servers = legacy_servers.into_iter().map(|ls| Server {
                name: ls.name,
                user: ls.user,
                host: ls.host,
                port: ls.port,
                auth_type: AuthType::Agent,
                group: "General".to_string(),
            }).collect();
            return Ok(Config { servers, master_password: None });
        }

        // 3. Encrypted Config
        let enc_config: EncryptedConfig = serde_json::from_str(&content).map_err(|e| {
             format!("Failed to parse config file at {:?}: {}", config_path, e)
        })?;
        
        println!("🔒 Encrypted configuration found. Please enter master password:");
        let password = rpassword::read_password()?;

        let salt = general_purpose::STANDARD.decode(&enc_config.salt)?;
        let nonce_bytes = general_purpose::STANDARD.decode(&enc_config.nonce)?;
        let ciphertext = general_purpose::STANDARD.decode(&enc_config.ciphertext)?;

        let key = derive_key(&password, &salt);
        let cipher = Aes256Gcm::new(&key.into());
        let nonce = Nonce::from_slice(&nonce_bytes);

        let plaintext = cipher.decrypt(nonce, ciphertext.as_ref())
            .map_err(|_| "Invalid password or corrupted data")?;

        let servers: Vec<Server> = serde_json::from_str(&String::from_utf8(plaintext)?)?;

        Ok(Config {
            servers,
            master_password: Some(password),
        })
    }

    pub fn save(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let config_path = get_config_path()?;
        if let Some(parent) = config_path.parent() {
            fs::create_dir_all(parent)?;
        }

        if self.master_password.is_none() {
            println!("🔒 Set a master password to encrypt your data:");
             let p1 = rpassword::read_password()?;
             println!("🔒 Confirm master password:");
             let p2 = rpassword::read_password()?;
             if p1 != p2 {
                 return Err("Passwords do not match".into());
             }
             self.master_password = Some(p1);
        }

        let password = self.master_password.as_ref().unwrap();
        let mut salt = [0u8; SALT_LEN];
        OsRng.fill_bytes(&mut salt);
        
        let key = derive_key(password, &salt);
        let cipher = Aes256Gcm::new(&key.into());
        
        let mut nonce_bytes = [0u8; NONCE_LEN];
        OsRng.fill_bytes(&mut nonce_bytes);
        let nonce = Nonce::from_slice(&nonce_bytes);

        let json = serde_json::to_string(&self.servers)?;
        let ciphertext = cipher.encrypt(nonce, json.as_bytes())
            .map_err(|_| "Encryption failed")?;

        let enc_config = EncryptedConfig {
            salt: general_purpose::STANDARD.encode(salt),
            nonce: general_purpose::STANDARD.encode(nonce_bytes),
            ciphertext: general_purpose::STANDARD.encode(ciphertext),
        };

        let content = serde_json::to_string_pretty(&enc_config)?;
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

    pub fn import_ssh_config(&mut self) -> Result<usize, Box<dyn std::error::Error>> {
        let ssh_dir = dirs::home_dir().ok_or("No home dir")?.join(".ssh");
        let config_path = ssh_dir.join("config");
        
        if !config_path.exists() {
            return Ok(0);
        }

        let mut reader = BufReader::new(fs::File::open(config_path)?);
        let config = SshConfig::default().parse(&mut reader, ssh2_config::ParseRule::ALLOW_UNKNOWN_FIELDS)?;
        
        let mut count = 0;
        
        let file_content = fs::read_to_string(ssh_dir.join("config"))?;
            
        for line in file_content.lines() {
            let line = line.trim();
            if line.starts_with("Host ") {
                let host_alias = line.trim_start_matches("Host ").trim();
                if host_alias.contains('*') { continue; } 
                
                let params = config.query(host_alias);
                
                let hostname = params.host_name.unwrap_or(host_alias.to_string());
                let user = params.user.unwrap_or(whoami::username());
                let port = params.port.unwrap_or(22);
                // identity_file is Option<Vec<PathBuf>>
                let identity = params.identity_file.and_then(|files| files.first().map(|p| p.to_string_lossy().to_string()));

                
                // Check duplicate
                if !self.servers.iter().any(|s| s.name == host_alias) {
                    self.servers.push(Server {
                        name: host_alias.to_string(),
                        user,
                        host: hostname,
                        port,
                        auth_type: if let Some(path) = identity {
                            AuthType::Key(path)
                        } else {
                            AuthType::Agent // Default to agent if no key specified but in config
                        },
                        group: "Imported".to_string(),
                    });
                    count += 1;
                }
            }
        }

        Ok(count)
    }
}

fn derive_key(password: &str, salt: &[u8]) -> [u8; 32] {
    let mut key = [0u8; 32];
    pbkdf2::<Hmac<Sha256>>(password.as_bytes(), salt, ITERATIONS, &mut key)
        .expect("PBKDF2 failed");
    key
}

fn get_config_path() -> Result<PathBuf, Box<dyn std::error::Error>> {
    let mut path = dirs::config_dir().ok_or("Could not find config directory")?;
    path.push("ssh-manager");
    path.push("servers.json");
    Ok(path)
}
