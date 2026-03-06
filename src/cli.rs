use crate::{app_state::AppState, storage, stats};
use std::sync::{Arc, RwLock};
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::sync::Notify;
use chrono::{Utc, Duration};

pub async fn run_cli(state: Arc<RwLock<AppState>>, notify: Arc<Notify>) {
    println!("Monitoring daemon started.");
    println!("Type 'help' for commands.");

    let stdin = BufReader::new(tokio::io::stdin());
    let mut lines = stdin.lines();

    while let Ok(Some(line)) = lines.next_line().await {
        let mut parts = line.split_whitespace();
        match parts.next() {
            Some("pause") => {
                if let Some(url) = parts.next() {
                    let mut site_url = String::new();
                    {
                        let mut lock = state.write().unwrap();
                        if let Some(s) = lock.sites.iter_mut().find(|s| s.url.contains(url)) {
                            s.paused = true; site_url = s.url.clone(); storage::save(&lock);
                        }
                    }
                    if !site_url.is_empty() { println!("Paused site: {}", site_url); }
                }
            }
            Some("resume") => {
                if let Some(url) = parts.next() {
                    let mut site_url = String::new();
                    {
                        let mut lock = state.write().unwrap();
                        if let Some(s) = lock.sites.iter_mut().find(|s| s.url.contains(url)) {
                            s.paused = false; site_url = s.url.clone(); storage::save(&lock);
                        }
                    }
                    if !site_url.is_empty() { println!("Resumed site: {}", site_url); }
                }
            }
            Some("silence") => {
                if let Some(mins_str) = parts.next() {
                    if let Ok(mins) = mins_str.parse::<i64>() {
                        {
                            let mut lock = state.write().unwrap();
                            lock.silence_until = Some(Utc::now() + Duration::minutes(mins));
                            storage::save(&lock);
                        }
                        println!("All notifications silenced for {} minutes.", mins);
                    }
                }
            }
            Some("stats") => { println!("{}", stats::get_report()); }
            Some("site") => {
                match parts.next() {
                    Some("frequency") => {
                        let url_part = parts.next();
                        let freq = parts.next();
                        if let (Some(u), Some(f)) = (url_part, freq) {
                            if let Ok(val) = f.parse::<u64>() {
                                let mut site_url = String::new();
                                {
                                    let mut lock = state.write().unwrap();
                                    if let Some(s) = lock.sites.iter_mut().find(|s| s.url.contains(u)) {
                                        s.frequency_multiplier = val;
                                        site_url = s.url.clone();
                                        storage::save(&lock);
                                    }
                                }
                                if !site_url.is_empty() { println!("Set check frequency for {} to {} checks per interval.", site_url, val); }
                            }
                        }
                    }
                    Some("recipient") => {
                        let cmd = parts.next();
                        let url_part = parts.next();
                        let phone = parts.next();
                        if let (Some(c), Some(u), Some(p)) = (cmd, url_part, phone) {
                            let mut site_url = String::new();
                            {
                                let mut lock = state.write().unwrap();
                                if let Some(s) = lock.sites.iter_mut().find(|s| s.url.contains(u)) {
                                    let list = s.recipients.get_or_insert_with(Vec::new);
                                    if c == "add" && !list.contains(&p.to_string()) { list.push(p.to_string()); }
                                    else if c == "rm" { list.retain(|x| x != p); }
                                    site_url = s.url.clone();
                                    storage::save(&lock);
                                }
                            }
                            if !site_url.is_empty() { println!("Updated recipients for {}", site_url); }
                        }
                    }
                    _ => println!("Usage: site frequency <url> <val> | site recipient [add|rm] <url> <num>"),
                }
            }
            Some("recipient") => {
                match parts.next() {
                    Some("add") => {
                        if let Some(phone) = parts.next() {
                            let mut lock = state.write().unwrap();
                            if !lock.whatsapp.recipients.contains(&phone.to_string()) {
                                lock.whatsapp.recipients.push(phone.to_string());
                                lock.whatsapp.enabled = true;
                                storage::save(&lock);
                                println!("Recipient {} added.", phone);
                            }
                        }
                    }
                    Some("rm") => {
                        if let Some(phone) = parts.next() {
                            let mut lock = state.write().unwrap();
                            lock.whatsapp.recipients.retain(|x| x != phone);
                            storage::save(&lock);
                            println!("Recipient {} removed.", phone);
                        }
                    }
                    Some("list") => {
                        let lock = state.read().unwrap();
                        println!("Authorized Recipients:");
                        for r in &lock.whatsapp.recipients { println!(" - {}", r); }
                    }
                    _ => println!("Usage: recipient [add|rm|list] <number>"),
                }
            }
            Some("setup") => match parts.next() {
                Some("name") => {
                    println!("Enter new daemon name:");
                    let name = read_line().await;
                    let mut lock = state.write().unwrap();
                    lock.name = name; storage::save(&lock);
                    println!("Daemon name updated to: {}", lock.name);
                }
                Some("retries") => {
                    if let Some(val) = parts.next() {
                        if let Ok(n) = val.parse::<u64>() {
                            let mut lock = state.write().unwrap();
                            lock.max_retries = n; storage::save(&lock);
                            println!("Global retry threshold set to: {}", n);
                        }
                    }
                }
                Some("telegram") => {
                    println!("Enter bot token:");
                    let token = read_line().await;
                    println!("Enter chat ID:");
                    let chat = read_line().await;
                    let mut lock = state.write().unwrap();
                    lock.telegram.enabled = true; lock.telegram.bot_token = Some(token); lock.telegram.chat_id = Some(chat);
                    storage::save(&lock); notify.notify_one();
                    println!("Telegram configured.");
                }
                Some("whatsapp") => {
                    println!("Enter recipient phone (e.g., 50612345678):");
                    let phone = read_line().await;
                    let mut lock = state.write().unwrap();
                    if !lock.whatsapp.recipients.contains(&phone) { lock.whatsapp.recipients.push(phone.clone()); }
                    lock.whatsapp.enabled = true; storage::save(&lock); notify.notify_one();
                    println!("WhatsApp configured.");
                }
                _ => println!("Usage: setup [name|retries|telegram|whatsapp]"),
            },
            Some("add") => {
                if let Some(url_raw) = parts.next() {
                    let url = if !url_raw.starts_with("http://") && !url_raw.starts_with("https://") { format!("https://{}", url_raw) } else { url_raw.to_string() };
                    let name = parts.next().map(|s| s.to_string());
                    let emoji = parts.next().map(|s| s.to_string());
                    let mut lock = state.write().unwrap();
                    lock.sites.push(crate::app_state::Site { url, name, emoji, timeout_seconds: 10, recipients: None, last_status: None, last_check: None, paused: false, frequency_multiplier: 1, consecutive_failures: 0 });
                    storage::save(&lock); notify.notify_one();
                    println!("Site added.");
                }
            }
            Some("rm") | Some("remove") => {
                if let Some(url) = parts.next() {
                    let mut lock = state.write().unwrap();
                    lock.sites.retain(|s| !s.url.contains(url));
                    storage::save(&lock); notify.notify_one(); println!("Site removed.");
                }
            }
            Some("list") => {
                let lock = state.read().unwrap();
                println!("Monitored Sites:");
                for s in &lock.sites {
                    let status = if s.paused { "PAUSED".to_string() } else { s.last_status.clone().unwrap_or_else(|| "Never checked".to_string()) };
                    let last_check = s.last_check.map(|dt| dt.with_timezone(&chrono::Local).format("%Y-%m-%d %H:%M:%S").to_string()).unwrap_or_else(|| "Never".to_string());
                    let freq = if s.frequency_multiplier > 1 { format!(" | Freq: {} checks/interval", s.frequency_multiplier) } else { "".to_string() };
                    println!(" - {} ({}){}", s.name.as_ref().unwrap_or(&s.url), s.url, freq);
                    println!("   Status: {} | Last Check: {}", status, last_check);
                }
            }
            Some("shutdown") | Some("quit") | Some("exit") => { break; }
            Some("help") => {
                println!("Available Commands:");
                println!("  stats                         - Show session uptime and counters");
                println!("  pause/resume <url_part>       - Toggle monitoring for a site");
                println!("  silence <minutes>             - Mute all notifications globally");
                println!("  site frequency <url> <val>    - How many times to check a site per global interval");
                println!("  site recipient add/rm <url> <num> - Targeted alerts for a site");
                println!("  setup retries <number>        - Global failure retry threshold");
                println!("  setup name/telegram/whatsapp  - General configuration");
                println!("  add <url> [name] [emoji]      - Add a new site");
                println!("  rm <url_part>                 - Remove a site");
                println!("  list                          - Show sites and current status");
                println!("  shutdown / quit / exit        - Stop the daemon");
            }
            _ => println!("Unknown command. Type 'help' for a list of commands."),
        }
    }
}

async fn read_line() -> String {
    let mut input = String::new();
    let stdin = tokio::io::stdin();
    let mut reader = BufReader::new(stdin);
    reader.read_line(&mut input).await.unwrap();
    input.trim().to_string()
}
