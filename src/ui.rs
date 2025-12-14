use crate::config::Server;
use dialoguer::{theme::ColorfulTheme, Input, Select};
use console::Term;

pub enum Action {
    Connect,
    AddServer,
    RemoveServer,
    ListServers,
    Exit,
}

pub fn main_menu() -> Action {
    let items = vec![
        "🚀 Connect to Server",
        "➕ Add New Server",
        "🗑️  Remove Server",
        "📋 List Servers",
        "🚪 Exit",
    ];

    let selection = Select::with_theme(&ColorfulTheme::default())
        .with_prompt("SSH Manager - Select an action")
        .default(0)
        .items(&items)
        .interact_on(&Term::stderr())
        .unwrap_or(4); 

    match selection {
        0 => Action::Connect,
        1 => Action::AddServer,
        2 => Action::RemoveServer,
        3 => Action::ListServers,
        _ => Action::Exit,
    }
}

pub fn add_server_prompt() -> Server {
    println!("📝 Enter server details:");
    
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

    Server {
        name,
        user,
        host,
        port,
    }
}

pub fn select_server(servers: &[Server]) -> Option<usize> {
    if servers.is_empty() {
        println!("⚠️  No servers found. Add one first!");
        return None;
    }

    let items: Vec<String> = servers
        .iter()
        .map(|s| format!("{} ({}@{}:{})", s.name, s.user, s.host, s.port))
        .collect();

    let selection = Select::with_theme(&ColorfulTheme::default())
        .with_prompt("Select a server")
        .default(0)
        .items(&items)
        .interact_on(&Term::stderr())
        .ok()?;

    Some(selection)
}
