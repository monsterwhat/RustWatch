use crate::notifier::Notifier;
use crate::app_state::{AppState, Site};
use crate::storage;
use crate::stats;
use async_trait::async_trait;
use anyhow::Result;
use std::sync::{Arc, RwLock};
use std::sync::atomic::{AtomicBool, Ordering};
use tokio::sync::Notify;
use chrono::{Utc, Duration};

use whatsapp_rust::bot::Bot;
use whatsapp_rust::client::Client;
use whatsapp_rust::store::SqliteStore;
use whatsapp_rust_tokio_transport::TokioWebSocketTransportFactory;
use whatsapp_rust_ureq_http_client::UreqHttpClient;
use wacore_binary::jid::Jid;
use waproto::whatsapp::Message;
use wacore::types::events::Event;

pub struct WhatsAppNotifier {
    client: Arc<Client>,
    recipients: Vec<String>,
    connected: Arc<std::sync::atomic::AtomicBool>,
}

impl WhatsAppNotifier {
    pub fn is_connected(&self) -> bool {
        self.connected.load(Ordering::SeqCst)
    }
    
    pub async fn send_startup_message(&self, name: &str) -> Result<()> {
        if !self.connected.load(Ordering::SeqCst) {
            return Err(anyhow::anyhow!("WhatsApp not connected"));
        }
        
        // Wait for device sync to complete
        for _ in 0..20 {
            tokio::time::sleep(std::time::Duration::from_millis(500)).await;
            if self.connected.load(Ordering::SeqCst) {
                break;
            }
        }
        
        // Additional buffer for encryption keys to stabilize
        tokio::time::sleep(std::time::Duration::from_millis(1500)).await;
        
        for recipient in &self.recipients {
            let jid: Jid = format!("{}@s.whatsapp.net", recipient).parse()?;
            send_robust(self.client.clone(), jid, format!("✅ {} successfully configured! (Remote commands active)", name)).await?;
        }
        Ok(())
    }
}

async fn send_robust(client: Arc<Client>, jid: Jid, text: String) -> Result<()> {
    let mut msg = Message::default();
    msg.conversation = Some(text);
    tokio::time::sleep(std::time::Duration::from_millis(500)).await;
    for i in 0..15 {
        match client.send_message(jid.clone(), msg.clone()).await {
            Ok(_) => return Ok(()),
            Err(e) if e.to_string().contains("not connected") => {
                if i == 0 { println!("Waiting for WhatsApp connection..."); }
                tokio::time::sleep(std::time::Duration::from_secs(1)).await;
            }
            Err(e) => return Err(e.into()),
        }
    }
    client.send_message(jid, msg).await?;
    Ok(())
}

async fn send_with_log(client: Arc<Client>, jid: Jid, text: String) {
    if let Err(e) = send_robust(client, jid, text).await {
        eprintln!("Failed to send WhatsApp message: {}", e);
    }
}

impl WhatsAppNotifier {
    pub async fn new(recipients: Vec<String>, state: Arc<RwLock<AppState>>, notify: Arc<Notify>) -> Result<Self> {
        let backend = Arc::new(SqliteStore::new("whatsapp.db").await?);
        let recipients_for_event = recipients.clone();
        let ready_notify = Arc::new(Notify::new());
        let ready_flag = Arc::new(AtomicBool::new(false));
        let ready_flag_clone = ready_flag.clone();
        let ready_notify_clone = ready_notify.clone();
        let connected = Arc::new(AtomicBool::new(false));
        let connected_for_event = connected.clone();

        let mut bot = Bot::builder()
            .with_backend(backend)
            .with_transport_factory(TokioWebSocketTransportFactory::new())
            .with_http_client(UreqHttpClient::new())
            .on_event(move |event, client| {
                let state = state.clone();
                let notify = notify.clone();
                let recipients = recipients_for_event.clone();
                let client = client.clone();
                let ready_flag_inner = ready_flag_clone.clone();
                let ready_notify_inner = ready_notify_clone.clone();
                let connected_inner = connected_for_event.clone();
                async move {
                    match event {
                        Event::PairingQrCode { code, .. } => {
                            let _ = qr2term::print_qr(&code);
                            println!("Raw QR: {}", code);
                        }
                        Event::PairSuccess(_) => {
                            println!("WhatsApp paired successfully!");
                        }
                        Event::Connected(_) => {
                            println!("🔄 WhatsApp reconnecting, waiting for sync...");
                            ready_flag_inner.store(false, Ordering::Relaxed);
                        }
                        Event::OfflineSyncCompleted(_) => {
                            if !ready_flag_inner.load(Ordering::Relaxed) {
                                ready_flag_inner.store(true, Ordering::Relaxed);
                                ready_notify_inner.notify_one();
                            }
                            connected_inner.store(true, Ordering::SeqCst);
                            println!("✓ WhatsApp connected and ready");
                        }
                        Event::DeviceListUpdate(_) => {
                            println!("📱 Device list updated - re-syncing keys...");
                            ready_flag_inner.store(false, Ordering::Relaxed);
                            connected_inner.store(false, Ordering::SeqCst);
                        }
                        Event::Disconnected(_) => {
                            println!("⚠️ WhatsApp disconnected!");
                            connected_inner.store(false, Ordering::SeqCst);
                            ready_flag_inner.store(false, Ordering::Relaxed);
                        }
                        Event::LoggedOut(_) => {
                            println!("🔴 WhatsApp session logged out! Please re-authenticate.");
                            connected_inner.store(false, Ordering::SeqCst);
                            ready_flag_inner.store(false, Ordering::Relaxed);
                        }
                        Event::StreamReplaced(_) => {
                            println!("⚠️ WhatsApp session replaced (another device logged in)");
                            connected_inner.store(false, Ordering::SeqCst);
                            ready_flag_inner.store(false, Ordering::Relaxed);
                        }
                        Event::Message(message, info) => {
                            let sender_full = info.source.sender.to_string();
                            let mut text_content: Option<String> = None;
                            if let Some(txt) = message.conversation.as_ref() { text_content = Some(txt.clone()); }
                            else if let Some(ext) = message.extended_text_message.as_ref() { if let Some(txt) = ext.text.as_ref() { text_content = Some(txt.clone()); } }
                            else if let Some(dev_sent) = message.device_sent_message.as_ref() {
                                if let Some(inner_msg) = dev_sent.message.as_ref() {
                                    if let Some(txt) = inner_msg.conversation.as_ref() { text_content = Some(txt.clone()); }
                                    else if let Some(ext) = inner_msg.extended_text_message.as_ref() { if let Some(txt) = ext.text.as_ref() { text_content = Some(txt.clone()); } }
                                }
                            }

                            let matched_recipient = recipients.iter().find(|r| sender_full.contains(*r));
                            if let Some(recipient_id) = matched_recipient {
                                if let Some(text) = text_content {
                                    let text = text.trim().to_string();
                                    let recipient_id = recipient_id.clone();
                                    let state = state.clone();
                                    let notify = notify.clone();
                                    let client = client.clone();
                                    let connected_inner = connected_inner.clone();

                                    tokio::spawn(async move {
                                        // Wait for connection to be ready
                                        for _ in 0..10 {
                                            if connected_inner.load(Ordering::SeqCst) {
                                                break;
                                            }
                                            tokio::time::sleep(std::time::Duration::from_millis(500)).await;
                                        }
                                        
                                        if !connected_inner.load(Ordering::SeqCst) {
                                            eprintln!("Cannot reply: WhatsApp not connected");
                                            return;
                                        }
                                        let reply_jid: Jid = match format!("{}@s.whatsapp.net", recipient_id).parse() {
                                            Ok(j) => j,
                                            Err(_) => return,
                                        };

                                        if text.to_lowercase().starts_with("add ") {
                                            let url_raw = text[4..].trim().to_string();
                                            let url = if !url_raw.starts_with("http://") && !url_raw.starts_with("https://") { format!("https://{}", url_raw) } else { url_raw.to_string() };
                                            {
                                                let mut lock = state.write().unwrap();
                                                lock.sites.push(Site { url: url.clone(), name: None, emoji: None, timeout_seconds: 10, recipients: None, last_status: None, last_check: None, paused: false, frequency_multiplier: 1, consecutive_failures: 0 });
                                                storage::save(&lock);
                                            }
                                            notify.notify_one();
                                            send_with_log(client, reply_jid, format!("✅ Added site: {}", url)).await;
                                        } else if text.to_lowercase().starts_with("rm ") || text.to_lowercase().starts_with("remove ") {
                                            let url_part = if text.to_lowercase().starts_with("rm ") { text[3..].trim().to_string() } else { text[7..].trim().to_string() };
                                            let mut removed_url = String::new();
                                            {
                                                let mut lock = state.write().unwrap();
                                                let old_len = lock.sites.len();
                                                lock.sites.retain(|s| !s.url.contains(&url_part));
                                                if lock.sites.len() < old_len { storage::save(&lock); removed_url = url_part; }
                                            }
                                            if !removed_url.is_empty() { notify.notify_one(); send_with_log(client, reply_jid, format!("✅ Removed site: {}", removed_url)).await; }
                                        } else if text.to_lowercase().starts_with("site frequency ") {
                                            let parts: Vec<String> = text.split_whitespace().map(|s| s.to_string()).collect();
                                            if parts.len() >= 4 {
                                                let url_part = &parts[2];
                                                if let Ok(f) = parts[3].parse::<u64>() {
                                                    let mut site_url = String::new();
                                                    {
                                                        let mut lock = state.write().unwrap();
                                                        if let Some(s) = lock.sites.iter_mut().find(|s| s.url.contains(url_part)) {
                                                            s.frequency_multiplier = f;
                                                            site_url = s.url.clone();
                                                            storage::save(&lock);
                                                        }
                                                    }
                                                    if !site_url.is_empty() { send_with_log(client, reply_jid, format!("✅ Freq for {} set to every {} cycles", site_url, f)).await; }
                                                }
                                            }
                                        } else if text.to_lowercase().starts_with("setup retries ") {
                                            if let Ok(n) = text[14..].trim().parse::<u64>() {
                                                {
                                                    let mut lock = state.write().unwrap();
                                                    lock.max_retries = n;
                                                    storage::save(&lock);
                                                }
                                                send_with_log(client, reply_jid, format!("✅ Max retries set to {}", n)).await;
                                            }
                                        } else if text.to_lowercase().starts_with("site recipient ") {
                                            let parts: Vec<String> = text.split_whitespace().map(|s| s.to_string()).collect();
                                            if parts.len() >= 5 {
                                                let cmd = &parts[2]; let url_part = &parts[3]; let phone = &parts[4];
                                                let mut site_url = String::new();
                                                {
                                                    let mut lock = state.write().unwrap();
                                                    if let Some(s) = lock.sites.iter_mut().find(|s| s.url.contains(url_part)) {
                                                        let list = s.recipients.get_or_insert_with(Vec::new);
                                                        if cmd == "add" && !list.contains(phone) { list.push(phone.clone()); }
                                                        else if cmd == "rm" { list.retain(|x| x != phone); }
                                                        site_url = s.url.clone(); storage::save(&lock);
                                                    }
                                                }
                                                if !site_url.is_empty() { send_with_log(client, reply_jid, format!("✅ Updated recipients for {}", site_url)).await; }
                                            }
                                        } else if text.to_lowercase().starts_with("pause ") {
                                            let url_part = text[6..].trim().to_string();
                                            let mut site_url = String::new();
                                            {
                                                let mut lock = state.write().unwrap();
                                                if let Some(s) = lock.sites.iter_mut().find(|s| s.url.contains(&url_part)) {
                                                    s.paused = true; site_url = s.url.clone(); storage::save(&lock);
                                                }
                                            }
                                            if !site_url.is_empty() { send_with_log(client, reply_jid, format!("⏸ Paused: {}", site_url)).await; }
                                        } else if text.to_lowercase().starts_with("resume ") {
                                            let url_part = text[7..].trim().to_string();
                                            let mut site_url = String::new();
                                            {
                                                let mut lock = state.write().unwrap();
                                                if let Some(s) = lock.sites.iter_mut().find(|s| s.url.contains(&url_part)) {
                                                    s.paused = false; site_url = s.url.clone(); storage::save(&lock);
                                                }
                                            }
                                            if !site_url.is_empty() { send_with_log(client, reply_jid, format!("▶ Resumed: {}", site_url)).await; }
                                        } else if text.to_lowercase().starts_with("silence ") {
                                            if let Ok(m) = text[8..].trim().parse::<i64>() {
                                                {
                                                    let mut lock = state.write().unwrap();
                                                    lock.silence_until = Some(Utc::now() + Duration::minutes(m));
                                                    storage::save(&lock);
                                                }
                                                send_with_log(client, reply_jid, format!("🔇 Silenced for {}m", m)).await;
                                            }
                                        } else if text.to_lowercase() == "stats" {
                                            send_with_log(client, reply_jid, stats::get_report()).await;
                                        } else if text.to_lowercase() == "list" {
                                            let sites_list = {
                                                let lock = state.read().unwrap();
                                                if lock.sites.is_empty() { "No sites monitored.".to_string() }
                                                else {
                                                    lock.sites.iter().map(|s| {
                                                        let status = if s.paused { "PAUSED".to_string() } else { s.last_status.clone().unwrap_or_else(|| "?".to_string()) };
                                                        let freq = if s.frequency_multiplier > 1 { format!(" ({}x)", s.frequency_multiplier) } else { "".to_string() };
                                                        format!("- {} [{}] {}", s.url, status, freq)
                                                    }).collect::<Vec<_>>().join("\n")
                                                }
                                            };
                                            send_with_log(client, reply_jid, format!("📋 Sites:\n{}", sites_list)).await;
                                        } else if text.to_lowercase() == "shutdown" || text.to_lowercase() == "quit" || text.to_lowercase() == "exit" {
                                            let name = { state.read().unwrap().name.clone() };
                                            send_with_log(client, reply_jid, format!("🛑 Shutting down {}...", name)).await;
                                            tokio::time::sleep(std::time::Duration::from_secs(2)).await;
                                            std::process::exit(0);
                                        } else if text.to_lowercase().starts_with("recipient ") {
                                            let parts: Vec<String> = text.split_whitespace().map(|s| s.to_string()).collect();
                                            if parts.len() >= 3 && parts[1] == "add" {
                                                let phone = parts[2].clone();
                                                let added = {
                                                    let mut lock = state.write().unwrap();
                                                    if !lock.whatsapp.recipients.contains(&phone) { lock.whatsapp.recipients.push(phone.clone()); storage::save(&lock); true } else { false }
                                                };
                                                if added { notify.notify_one(); send_with_log(client, reply_jid, format!("✅ Added recipient: {}", phone)).await; }
                                            } else if parts.len() >= 3 && parts[1] == "rm" {
                                                let phone = parts[2].clone();
                                                { let mut lock = state.write().unwrap(); lock.whatsapp.recipients.retain(|r| r != &phone); storage::save(&lock); }
                                                notify.notify_one();
                                                send_with_log(client, reply_jid, format!("✅ Removed recipient: {}", phone)).await;
                                            } else if parts.len() >= 2 && parts[1] == "list" {
                                                let recipients_list = { state.read().unwrap().whatsapp.recipients.iter().map(|r| format!("- {}", r)).collect::<Vec<_>>().join("\n") };
                                                send_with_log(client, reply_jid, format!("📋 Authorized Recipients:\n{}", recipients_list)).await;
                                            }
                                        } else if text.to_lowercase() == "help" {
                                            let name = { state.read().unwrap().name.clone() };
                                            let msg = format!(
                                                "🤖 {} Commands:\n- stats: Session report\n- setup retries <n>: Failure threshold\n- site frequency <url> <n>: n checks per interval\n- pause/resume <url>: Mute site\n- silence <min>: Global mute\n- site recipient add/rm <url> <num>: Targeted alerts\n- add <url>: Add site\n- list: Show all\n- help: Commands",
                                                name
                                            );
                                            send_with_log(client, reply_jid, msg).await;
                                        }
                                    });
                                }
                            }
                        }
                        _ => {}
                    }
                }
            })
            .build()
            .await?;

        let client = bot.client();
        let connected_for_reconnect = connected.clone();
        let ready_flag_for_reconnect = ready_flag.clone();
        
        // Spawn background task to handle reconnection waits
        tokio::spawn(async move {
            loop {
                tokio::time::sleep(std::time::Duration::from_secs(1)).await;
                if !connected_for_reconnect.load(Ordering::SeqCst) {
                    // Disconnected, wait for reconnect
                    println!("⏳ Waiting for WhatsApp reconnection...");
                    loop {
                        tokio::time::sleep(std::time::Duration::from_millis(500)).await;
                        if ready_flag_for_reconnect.load(Ordering::Relaxed) {
                            connected_for_reconnect.store(true, Ordering::SeqCst);
                            println!("✓ WhatsApp reconnected and synced!");
                            break;
                        }
                    }
                }
            }
        });
        
        let run_handle = bot.run().await?;
        tokio::spawn(async move { let _ = run_handle.await; });
        
        println!("Waiting for WhatsApp device sync to complete...");
        tokio::select! {
            _ = ready_notify.notified() => {
                println!("WhatsApp device sync complete!");
                connected.store(true, Ordering::SeqCst);
            }
            _ = tokio::time::sleep(std::time::Duration::from_secs(5)) => {
                println!("Warning: Sync timeout, proceeding anyway...");
            }
        }
        
        Ok(Self { client, recipients, connected })
    }
}

#[async_trait]
impl Notifier for WhatsAppNotifier {
    async fn send(&self, message: &str, target_recipients: Option<&[String]>) -> Result<()> {
        if !self.connected.load(Ordering::SeqCst) {
            return Err(anyhow::anyhow!("WhatsApp not connected"));
        }
        
        let list_to_send = if let Some(targets) = target_recipients { self.recipients.iter().filter(|r| targets.contains(r)).cloned().collect::<Vec<_>>() } else { self.recipients.clone() };
        for recipient in &list_to_send {
            let jid: Jid = format!("{}@s.whatsapp.net", recipient).parse().map_err(|e| anyhow::anyhow!("Invalid JID: {}", e))?;
            send_robust(self.client.clone(), jid, message.to_string()).await?;
        }
        Ok(())
    }
}
