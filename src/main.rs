mod config;
mod ui;

use config::{Config, AuthType, Server};
use std::net::TcpStream;
use std::io::{Read, Write};
use std::thread;
use std::sync::mpsc;
use crossterm::terminal::{enable_raw_mode, disable_raw_mode};
use ssh2::Session;
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
    #[tabled(rename = "Auth")]
    auth_mode: String,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut config = Config::load()?;

    loop {
        match ui::main_menu() {
            ui::Action::Connect => {
                if let Some(index) = ui::select_server(&config.servers) {
                    let server = &config.servers[index];
                    println!("🚀 Connecting to {} ({}@{})...", server.name, server.user, server.host);
                    
                    if let Err(e) = connect_ssh(server) {
                         println!("❌ Connection failed: {}", e);
                    }
                    
                    // Ensure raw mode is disabled if it wasn't already
                    let _ = disable_raw_mode();
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
                        auth_mode: match &s.auth_type {
                            AuthType::Password(_) => "🔑 Password".to_string(),
                            AuthType::Key(_) => "🗝️ Key".to_string(),
                            AuthType::Agent => "🕵️ Agent".to_string(),
                        },
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

fn connect_ssh(server: &Server) -> Result<(), Box<dyn std::error::Error>> {
    let tcp = TcpStream::connect(format!("{}:{}", server.host, server.port))?;
    let mut sess = Session::new()?;
    sess.set_tcp_stream(tcp);
    sess.handshake()?;

    match &server.auth_type {
        AuthType::Password(p) => sess.userauth_password(&server.user, p)?,
        AuthType::Key(p) => sess.userauth_pubkey_file(&server.user, None, std::path::Path::new(p), None)?,
        AuthType::Agent => {
            sess.userauth_agent(&server.user)?;
        }
    }

    if !sess.authenticated() {
        return Err("Authentication failed".into());
    }

    let mut channel = sess.channel_session()?;
    
    // Create a PTY
    channel.request_pty("xterm-256color", None, None)?;
    channel.shell()?;

    enable_raw_mode()?;

    // Set non-blocking to handle IO loop
    sess.set_blocking(false);

    // Channel for Stdin -> Main Thread
    let (tx, rx) = mpsc::channel();
    
    // Spawn thread to read Stdin
    // We use a separate thread because Stdin::read is blocking
    thread::spawn(move || {
        let mut stdin = std::io::stdin();
        let mut buf = [0u8; 1]; // Read byte by byte for responsiveness
        loop {
            match stdin.read(&mut buf) {
                Ok(1) => {
                    if tx.send(buf[0]).is_err() {
                        break; 
                    }
                }
                Ok(_) => break, // EOF
                Err(_) => break,
            }
        }
    });

    let mut buf = [0u8; 2048];
    let mut stdout = std::io::stdout();

    loop {
        // 1. Write Stdin -> SSH
        while let Ok(byte) = rx.try_recv() {
            // Write to channel
            // Note: In non-blocking mode, write might error with WouldBlock?
            // Usually internal buffer handles small writes.
            // ssh2::Channel doesn't explicitly guarantee infinite buffering but for 1 byte it's fine.
            let _ = channel.write(&[byte]);
        }

        // 2. Read SSH -> Stdout
        match channel.read(&mut buf) {
            Ok(0) => {
                // EOF from server?
                if channel.eof() {
                   break;
                }
            }
            Ok(n) => {
                stdout.write_all(&buf[..n])?;
                stdout.flush()?;
            }
             Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                // No data
            },
            Err(e) => return Err(e.into()),
        }

        if channel.eof() {
            break;
        }
        
        // Small sleep to prevent 100% CPU
        thread::sleep(std::time::Duration::from_millis(5));
    }
    
    // Cleanup
    let _ = channel.close();
    let _ = channel.wait_close();
    disable_raw_mode()?;

    Ok(())
}
