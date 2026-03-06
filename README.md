# RustWatch 🚀

A robust, Rust-based website monitoring daemon that sends instant alerts via **WhatsApp** and **Telegram**. Features an interactive CLI and full remote control via WhatsApp.

## ✨ Features

- **Multi-Service Alerts:** Simultaneous notifications via WhatsApp and Telegram.
- **Targeted Notifications:** Route specific site alerts to specific phone numbers.
- **Remote Control:** Manage the daemon (add/remove sites, pause alerts, get stats) directly from your WhatsApp chat.
- **Precision Frequency:** Configurable check rates per site (e.g., check one site every 10 seconds while others stay at 5 minutes).
- **Intelligent Retries:** Global failure threshold to ignore temporary network blips.
- **Silence Mode:** Globally mute notifications for a set duration (DND).
- **Persistent State:** All configurations and last-known statuses are saved to `app_state.json`.

## 🚀 Getting Started

### Prerequisites
- [Rust](https://www.rust-lang.org/tools/install) (2024 edition)
- SQLite (The daemon uses a local `whatsapp.db` to manage sessions)

### Installation
1. Clone the repository

2. Build the project:
   ```bash
   cargo build --release
   ```
3. Run the daemon:
   ```bash
   ./target/release/monitor-daemon
   ```

### First-Time Setup
1. Run `setup name` to name your daemon (e.g., "Home Server").
2. Run `setup whatsapp` and follow the prompts.
3. **Scan the QR Code:** A QR code will appear in your terminal. Scan it with your WhatsApp mobile app (Linked Devices) to authorize the bot.
4. (Optional) Run `setup telegram` if you wish to enable Telegram alerts.

---

## 🛠 Command Reference

The following commands work in both the **CLI terminal** and via **WhatsApp message** (from an authorized number).

### Site Management
| Command | Description |
| :--- | :--- |
| `add <url> [name] [emoji]` | Add a new site to monitor. |
| `rm <url_part>` | Remove a site from the list. |
| `list` | Show all sites, current HTTP status, and last check time. |
| `pause <url_part>` | Temporarily stop monitoring a specific site. |
| `resume <url_part>` | Resume monitoring a paused site. |

### Advanced Config
| Command | Description |
| :--- | :--- |
| `site frequency <url> <n>` | Set check rate to **N times** per global interval. |
| `setup retries <n>` | Wait for **N consecutive failures** before sending a "DOWN" alert. |
| `silence <mins>` | Mute all outgoing notifications for X minutes. |
| `stats` | Show session uptime, total checks, and failure counts. |

### Recipient Management
| Command | Description |
| :--- | :--- |
| `recipient add <phone>` | Add a global phone number for all alerts. |
| `site recipient add <url> <p>` | Add a specific phone number to alerts for only one site. |
| `recipient list` | List all authorized global recipients. |

--- 

## 🔒 Security
- This tool stores WhatsApp session keys locally in `whatsapp.db`. **Never share this file.**
- Use `recipient` commands to ensure only your trusted numbers can execute remote commands.

## ⚖️ Legal Disclaimer

This is an **unofficial** tool and is not affiliated, associated, authorized, endorsed by, or in any way officially connected with WhatsApp or Meta Platforms, Inc. 

Use this tool at your own risk. Using third-party WhatsApp libraries may violate WhatsApp's Terms of Service and could potentially lead to account suspension. The author is not responsible for any consequences resulting from the use of this software.

## 🙏 Acknowledgments

This project is built upon the excellent work of:
- [whatsapp-rust](https://github.com/joaolopesi/whatsapp-rust) by João Lucas de Oliveira Lopes.

## 📄 License
This project is open-source and available under the MIT License.
