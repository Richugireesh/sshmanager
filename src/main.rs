mod config;
mod ui;

use config::{Config, AuthType, Server};
use std::net::TcpStream;
use std::io::{Read, Write};
use std::thread;
use std::sync::mpsc;
use std::path::Path;
use std::fs::File;
use crossterm::terminal::{enable_raw_mode, disable_raw_mode};
use ssh2::Session;
use tabled::{Table, Tabled};
use indicatif::{ProgressBar, ProgressStyle};

// Wrapper for Tabled to print Server nicely
#[derive(Tabled)]
struct ServerDisplay {
    #[tabled(rename = "Group")]
    group: String,
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
                    
                    match create_session(server) {
                        Ok(sess) => {
                             if let Err(e) = run_shell(sess) {
                                 println!("❌ Connection failed: {}", e);
                             }
                        },
                        Err(e) => println!("❌ Connection failed: {}", e),
                    }
                    
                    let _ = disable_raw_mode();
                    println!("\nPress Enter to continue...");
                    let _ = std::io::stdin().read_line(&mut String::new());
                }
            }
            ui::Action::FileTransfer => {
                if let Some(index) = ui::select_server(&config.servers) {
                    let server = &config.servers[index];
                    println!("🚀 Connecting to {} for SFTP...", server.name);

                     match create_session(server) {
                        Ok(sess) => {
                             if let Err(e) = run_sftp(sess) {
                                 println!("❌ SFTP failed: {}", e);
                             }
                        },
                        Err(e) => println!("❌ Connection failed: {}", e),
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
            ui::Action::ImportConfig => {
                println!("📥 Importing servers from ~/.ssh/config...");
                match config.import_ssh_config() {
                    Ok(count) => {
                        config.save()?;
                        println!("✅ Imported {} servers.", count);
                    },
                    Err(e) => println!("❌ Import failed: {}", e),
                }
                std::thread::sleep(std::time::Duration::from_millis(2000));
            }
            ui::Action::ListServers => {
                if config.servers.is_empty() {
                    println!("⚠️  No servers found.");
                } else {
                    let display_list: Vec<ServerDisplay> = config.servers.iter().map(|s| ServerDisplay {
                        group: s.group.clone(),
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

fn create_session(server: &Server) -> Result<Session, Box<dyn std::error::Error>> {
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
    
    Ok(sess)
}

fn run_shell(sess: Session) -> Result<(), Box<dyn std::error::Error>> {
    let mut channel = sess.channel_session()?;
    channel.request_pty("xterm-256color", None, None)?;
    channel.shell()?;

    enable_raw_mode()?;
    sess.set_blocking(false);

    let (tx, rx) = mpsc::channel();
    
    thread::spawn(move || {
        let mut stdin = std::io::stdin();
        let mut buf = [0u8; 1];
        loop {
            match stdin.read(&mut buf) {
                Ok(1) => { if tx.send(buf[0]).is_err() { break; } }
                Ok(_) => break,
                Err(_) => break,
            }
        }
    });

    let mut buf = [0u8; 2048];
    let mut stdout = std::io::stdout();

    loop {
        while let Ok(byte) = rx.try_recv() {
            let _ = channel.write(&[byte]);
        }

        match channel.read(&mut buf) {
            Ok(0) => { if channel.eof() { break; } }
            Ok(n) => { stdout.write_all(&buf[..n])?; stdout.flush()?; }
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {},
            Err(e) => return Err(e.into()),
        }

        if channel.eof() { break; }
        thread::sleep(std::time::Duration::from_millis(5));
    }
    
    let _ = channel.close();
    let _ = channel.wait_close();
    disable_raw_mode()?;
    Ok(())
}

fn run_sftp(sess: Session) -> Result<(), Box<dyn std::error::Error>> {
    let sftp = sess.sftp()?;
    let direction = ui::file_transfer_menu();
    
    match direction {
        ui::TransferDirection::Upload => {
            let local_path = ui::get_local_path("Local file path");
            let remote_path = ui::get_remote_path("Remote destination path");
            
            let mut file = File::open(&local_path)?;
            let file_size = file.metadata()?.len();
            
            let mut remote_file = sftp.create(Path::new(&remote_path))?;
            
            let pb = ProgressBar::new(file_size);
            pb.set_style(ProgressStyle::default_bar()
                .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {bytes}/{total_bytes} ({eta})")
                .unwrap()
                .progress_chars("#>-"));
            
            let mut buffer = [0u8; 8192]; // 8KB chunks
            loop {
                let n = file.read(&mut buffer)?;
                if n == 0 { break; }
                remote_file.write_all(&buffer[..n])?;
                pb.inc(n as u64);
            }
            pb.finish_with_message("Upload complete");
        },
        ui::TransferDirection::Download => {
            let remote_path = ui::get_remote_path("Remote file path");
            let local_path = ui::get_local_path("Local destination path");
            
            let mut remote_file = sftp.open(Path::new(&remote_path))?;
            let file_stat = remote_file.stat()?;
            let file_size = file_stat.size.unwrap_or(0);
            
            let mut file = File::create(&local_path)?;
            
            let pb = ProgressBar::new(file_size);
            pb.set_style(ProgressStyle::default_bar()
                .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {bytes}/{total_bytes} ({eta})")
                .unwrap()
                .progress_chars("#>-"));
                
            let mut buffer = [0u8; 8192];
            loop {
                let n = remote_file.read(&mut buffer)?;
                if n == 0 { break; }
                file.write_all(&buffer[..n])?;
                pb.inc(n as u64);
            }
            pb.finish_with_message("Download complete");
        }
    }
    
    Ok(())
}
