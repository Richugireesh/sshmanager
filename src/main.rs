mod config;
mod ui_render;
mod ui;
mod app;
mod tui;

use config::{Config, AuthType, Server};
use app::{App, InputMode, Focus, FormFocus};
use std::net::TcpStream;
use std::io::{Read, Write};
use std::thread;
use std::sync::mpsc;
use crossterm::event::{self, Event, KeyCode};
use ssh2::Session;
use crossterm::terminal::{enable_raw_mode, disable_raw_mode};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut config = Config::load()?;
    let mut terminal = tui::init()?;
    let mut app = App::new(config.servers.clone());

    loop {
        terminal.draw(|f| ui_render::ui(f, &mut app))?;

        if let Event::Key(key) = event::read()? {
            match app.input_mode {
                InputMode::Normal => match key.code {
                    KeyCode::Char('q') => app.should_quit = true,
                    KeyCode::Char('j') | KeyCode::Down => app.next(),
                    KeyCode::Char('k') | KeyCode::Up => app.previous(),
                    KeyCode::Char('a') => app.open_add_server_popup(),
                    KeyCode::Enter => {
                         if let Some(i) = app.list_state.selected() {
                             app.should_connect = Some(i);
                         }
                    },
                    KeyCode::Char('t') => {
                         if let Some(i) = app.list_state.selected() {
                             tui::restore()?;
                             let server = &app.servers[i];
                             println!("🚀 Connecting to {} for SFTP...", server.name);
                             
                             match create_session(server) {
                                Ok(sess) => {
                                     if let Err(e) = run_sftp(sess) {
                                         println!("❌ SFTP failed: {}", e);
                                     }
                                },
                                Err(e) => println!("❌ Connection failed: {}", e),
                             }
                             
                             println!("\nPress Enter to return to dashboard...");
                             let _ = std::io::stdin().read_line(&mut String::new());
                             terminal = tui::init()?;
                             terminal.clear()?;
                         }
                    },
                    KeyCode::Char('i') => {
                         if let Ok(count) = config.import_ssh_config() {
                             if count > 0 {
                                 config.save()?;
                                 app.servers = config.servers.clone();
                             }
                         }
                    },
                    // TODO: Implement Delete (d)
                    _ => {}
                },
                InputMode::Editing => {
                    match key.code {
                        KeyCode::Esc => app.close_popup(),
                        KeyCode::Tab => app.next_form_field(),
                        KeyCode::Enter => {
                             if let Focus::Form(FormFocus::Submit) = app.focus {
                                 app.save_server();
                                 config.servers = app.servers.clone();
                                 config.save()?;
                             } else {
                                 // Or just move next?
                                 app.next_form_field();
                             }
                        }
                        KeyCode::Left => {
                             if let Focus::Form(FormFocus::AuthType) = app.focus {
                                 if app.auth_type_idx > 0 { app.auth_type_idx -= 1; }
                             } else {
                                 // Let textarea handle
                                 handle_textarea_input(&key, &mut app);
                             }
                        },
                        KeyCode::Right => {
                             if let Focus::Form(FormFocus::AuthType) = app.focus {
                                 if app.auth_type_idx < 2 { app.auth_type_idx += 1; }
                             } else {
                                 handle_textarea_input(&key, &mut app);
                             }
                        }
                        _ => handle_textarea_input(&key, &mut app),
                    }
                }
            }
        }

        if app.should_quit {
            break;
        }

        if let Some(index) = app.should_connect {
            // Restore terminal for SSH session
            tui::restore()?;
            let server = &app.servers[index];
            println!("🚀 Connecting to {}...", server.name);
            
            // ... connect logic ...
            match create_session(server) {
                Ok(sess) => {
                     if let Err(e) = run_shell(sess) {
                         println!("❌ Connection failed: {}", e);
                         thread::sleep(std::time::Duration::from_secs(2));
                     }
                },
                Err(e) => {
                    println!("❌ Connection failed: {}", e);
                    thread::sleep(std::time::Duration::from_secs(2));
                }
            }

            // Re-init TUI
            terminal = tui::init()?;
            app.should_connect = None;
            terminal.clear()?;
        }
    }

    tui::restore()?;
    Ok(())
}

fn handle_textarea_input(key: &crossterm::event::KeyEvent, app: &mut App) {
    // Helper to dispatch input to active textarea
    use tui_textarea::Input;
    let input = Input::from(key.clone());
    
    match app.focus {
        Focus::Form(FormFocus::Group) => { app.group_input.input(input); },
        Focus::Form(FormFocus::Name) => { app.name_input.input(input); },
        Focus::Form(FormFocus::User) => { app.user_input.input(input); },
        Focus::Form(FormFocus::Host) => { app.host_input.input(input); },
        Focus::Form(FormFocus::Port) => { app.port_input.input(input); },
        Focus::Form(FormFocus::PasswordOrKey) => { app.password_key_input.input(input); },
        _ => {}
    }
}

// ... Original create_session and run_shell functions ...

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
    use indicatif::{ProgressBar, ProgressStyle};
    use std::path::Path;
    use std::fs::File;
    
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
            
            let mut buffer = [0u8; 8192];
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
