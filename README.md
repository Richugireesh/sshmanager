# Rust SSH Manager 🦀

A beautiful, secure, and interactive CLI tool to manage your SSH connections. Built with Rust.

![License](https://img.shields.io/badge/license-MIT-blue.svg)
![Rust](https://img.shields.io/badge/rust-1.70%2B-orange.svg)

## ✨ Features

- **🖥️ TUI Dashboard**: Modern, interactive terminal interface using `ratatui`.
- **🔐 Encrypted Storage**: All server details and passwords are safely encrypted using AES-256-GCM.
- **🚀 Native SSH Client**: Connects directly using the `ssh2` library.
- **📂 Groups & Organization**: Organize your servers into custom groups (e.g., Prod, Dev, Staging).
- **📥 Import Support**: Press `i` to import hosts from `~/.ssh/config`.
- **📂 SFTP Support**: Press `t` to Upload/Download files.
- **💅 Beautiful UI**: Rich terminal interface with colors and intuitive navigation.
- **🔑 Multiple Auth Methods**: Supports Password, SSH Key (Identity File), and SSH Agent.

## 📦 Installation

Ensure you have Rust installed. Clone the repository and build:

```bash
git clone https://github.com/richugireesh/ssh-manager.git
cd ssh-manager
cargo build --release
```

## 🚀 Usage

Run the application:

```bash
cargo run --release
```

### First Run
On the first launch, you will be prompted to set a **Master Password**. This password is used to encrypt your configuration file (`~/.config/ssh-manager/servers.json`). **Do not forget it!**

### Main Menu

- **Connect**: Browse and search your servers to connect.
- **Add Server**: Interactively add a new server.
- **Remove Server**: Delete a server from the list.
- **Import**: Scan `~/.ssh/config` for hosts.

### Keyboard Shortcuts

- **Arrow Keys**: Navigate menus.
- **Enter**: Select / Confirm.
- **Type**: Filter lists (Fuzzy Search).

## 🛠️ Tech Stack

- **[ssh2](https://crates.io/crates/ssh2)**: Native SSH implementation.
- **[dialoguer](https://crates.io/crates/dialoguer)** & **[console](https://crates.io/crates/console)**: Interactive CLI prompts.
- **[aes-gcm](https://crates.io/crates/aes-gcm)**: Authenticated encryption.
- **[serde](https://crates.io/crates/serde)**: Configuration serialization.
- **[tabled](https://crates.io/crates/tabled)**: Pretty tables.

## 📝 License

This project is licensed under the MIT License.
