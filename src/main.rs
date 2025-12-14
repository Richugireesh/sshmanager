mod config;
mod ui;

use config::Config;
use std::process::Command;
use tabled::{Table, Tabled};

// Wrapper for Tabled to print Server nicely
#[derive(Tabled)]
struct ServerDisplay {
    #[tabled(rename = "Alias")]
    name: String,
    #[tabled(rename = "User")]
    user: String,
    #[tabled(rename = "Host")]
    host: String,
    #[tabled(rename = "Port")]
    port: u16,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut config = Config::load()?;

    loop {
        match ui::main_menu() {
            ui::Action::Connect => {
                if let Some(index) = ui::select_server(&config.servers) {
                    let server = &config.servers[index];
                    println!("🚀 Connecting to {} ({}@{})...", server.name, server.user, server.host);
                    
                    // We need to use std::process::Command with .status() to inherit stdio
                    let status = Command::new("ssh")
                        .arg("-p")
                        .arg(server.port.to_string())
                        .arg(format!("{}@{}", server.user, server.host))
                        .status();

                    match status {
                        Ok(s) => {
                            if !s.success() {
                                println!("❌ SSH connection exited with error code: {:?}", s.code());
                            }
                        },
                        Err(e) => println!("❌ Failed to execute ssh: {}", e),
                    }
                    
                    println!("\nPress Enter to continue...");
                    let _ = std::io::stdin().read_line(&mut String::new());
                }
            }
            ui::Action::AddServer => {
                let server = ui::add_server_prompt();
                config.add_server(server);
                config.save()?;
                println!("✅ Server added successfully!");
                std::thread::sleep(std::time::Duration::from_millis(1000));
            }
            ui::Action::RemoveServer => {
                if let Some(index) = ui::select_server(&config.servers) {
                    config.remove_server(index);
                    config.save()?;
                    println!("🗑️  Server removed.");
                    std::thread::sleep(std::time::Duration::from_millis(1000));
                }
            }
            ui::Action::ListServers => {
                if config.servers.is_empty() {
                    println!("⚠️  No servers found.");
                } else {
                    let display_list: Vec<ServerDisplay> = config.servers.iter().map(|s| ServerDisplay {
                        name: s.name.clone(),
                        user: s.user.clone(),
                        host: s.host.clone(),
                        port: s.port,
                    }).collect();
                    
                    println!("{}", Table::new(display_list).to_string());
                }
                println!("\nPress Enter to continue...");
                let _ = std::io::stdin().read_line(&mut String::new());
            }
            ui::Action::Exit => {
                println!("👋 Bye!");
                break;
            }
        }
    }

    Ok(())
}
