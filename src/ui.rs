use crate::config::{Server, AuthType};
use dialoguer::{theme::ColorfulTheme, Input, Select, Password, FuzzySelect};
use console::Term;

pub enum Action {
    Connect,
    AddServer,
    RemoveServer,
    ListServers,
    ImportConfig,
    FileTransfer,
    Exit,
}

pub enum TransferDirection {
    Upload,
    Download,
}

pub fn main_menu() -> Action {
    let items = vec![
        "🚀 Connect to Server",
        "➕ Add New Server",
        "🗑️  Remove Server",
        "📋 List Servers",
        "📥 Import from SSH Config",
        "📂 File Transfer (SFTP)",
        "🚪 Exit",
    ];

    let selection = Select::with_theme(&ColorfulTheme::default())
        .with_prompt("SSH Manager - Select an action")
        .default(0)
        .items(&items)
        .interact_on(&Term::stderr())
        .unwrap_or(6); 

    match selection {
        0 => Action::Connect,
        1 => Action::AddServer,
        2 => Action::RemoveServer,
        3 => Action::ListServers,
        4 => Action::ImportConfig,
        5 => Action::FileTransfer,
        _ => Action::Exit,
    }
}

pub fn file_transfer_menu() -> TransferDirection {
    let items = vec!["⬆️  Upload (Local -> Remote)", "⬇️  Download (Remote -> Local)"];
    let selection = Select::with_theme(&ColorfulTheme::default())
        .with_prompt("Select transfer direction")
        .default(0)
        .items(&items)
        .interact()
        .unwrap();

    match selection {
        0 => TransferDirection::Upload,
        _ => TransferDirection::Download,
    }
}

pub fn get_local_path(prompt: &str) -> String {
    Input::with_theme(&ColorfulTheme::default())
        .with_prompt(prompt)
        .interact_text()
        .unwrap()
}

pub fn get_remote_path(prompt: &str) -> String {
    Input::with_theme(&ColorfulTheme::default())
        .with_prompt(prompt)
        .interact_text()
        .unwrap()
}

pub fn add_server_prompt() -> Server {
    println!("📝 Enter server details:");
    
    let group: String = Input::with_theme(&ColorfulTheme::default())
        .with_prompt("Group")
        .default("General".to_string())
        .interact_text()
        .unwrap();

    let name: String = Input::with_theme(&ColorfulTheme::default())
        .with_prompt("Server Name (alias)")
        .interact_text()
        .unwrap();

    let user: String = Input::with_theme(&ColorfulTheme::default())
        .with_prompt("Username")
        .interact_text()
        .unwrap();

    let host: String = Input::with_theme(&ColorfulTheme::default())
        .with_prompt("Host (IP or domain)")
        .interact_text()
        .unwrap();

    let port: u16 = Input::with_theme(&ColorfulTheme::default())
        .with_prompt("Port")
        .default(22)
        .interact_text()
        .unwrap();

    let auth_modes = vec!["Password", "SSH Key", "SSH Agent (No auth stored)"];
    let auth_selection = Select::with_theme(&ColorfulTheme::default())
        .with_prompt("Authentication Method")
        .default(0)
        .items(&auth_modes)
        .interact()
        .unwrap();

    let auth_type = match auth_selection {
        0 => {
             let pass = Password::with_theme(&ColorfulTheme::default())
                .with_prompt("Password")
                .interact()
                .unwrap();
             AuthType::Password(pass)
        },
        1 => {
            let key_path: String = Input::with_theme(&ColorfulTheme::default())
                .with_prompt("Path to Private Key")
                .default("~/.ssh/id_rsa".to_string())
                .interact_text()
                .unwrap();
            AuthType::Key(key_path)
        },
        _ => AuthType::Agent,
    };

    Server {
        name,
        user,
        host,
        port,
        auth_type,
        group,
    }
}

pub fn select_server(servers: &[Server]) -> Option<usize> {
    if servers.is_empty() {
        println!("⚠️  No servers found. Add one first!");
        return None;
    }

    let items: Vec<String> = servers
        .iter()
        .map(|s| format!("[{}] {} ({}@{}:{})", s.group, s.name, s.user, s.host, s.port))
        .collect();

    let selection = FuzzySelect::with_theme(&ColorfulTheme::default())
        .with_prompt("Select a server (Type to search)")
        .default(0)
        .items(&items)
        .interact_on(&Term::stderr())
        .ok()?;

    Some(selection)
}
