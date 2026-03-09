mod app_state;
mod storage;
mod state;
mod checker;
mod notifier;
mod telegram;
mod whatsapp;
mod notifier_manager;
mod cli;
mod stats;

use std::sync::{Arc, RwLock};
use tokio::sync::Notify;
use reqwest::Client;
use state::Tracker;
use chrono::Utc;

#[tokio::main]
async fn main() {
    if let Err(e) = run().await {
        eprintln!("CRITICAL ERROR: {:?}", e);
        tokio::time::sleep(std::time::Duration::from_secs(10)).await;
    }
}

async fn run() -> anyhow::Result<()> {
    println!("Initializing app state...");
    let state = Arc::new(RwLock::new(storage::load()));
    let notify = Arc::new(Notify::new());

    println!("Starting monitor loop...");
    tokio::spawn(monitor_loop(state.clone(), notify.clone()));

    println!("Starting CLI...");
    cli::run_cli(state.clone(), notify.clone()).await;

    println!("CLI exited.");
    Ok(())
}

async fn monitor_loop(state: Arc<RwLock<app_state::AppState>>, notify: Arc<Notify>) {
    let client = Client::new();
    let mut tracker = Tracker::new();
    let mut notifier_manager = notifier_manager::NotifierManager::new(state.clone(), notify.clone());
    let mut first_run = true;

    // Use a high-frequency tick (every 5 seconds) to check if any site is "due"
    let mut ticker = tokio::time::interval(std::time::Duration::from_secs(5));

    loop {
        tokio::select! {
            _ = ticker.tick() => {
                let snapshot = state.read().unwrap().clone();
                let notifiers = notifier_manager.get_notifiers(&snapshot).await;

                if first_run && !notifiers.is_empty() {
                    for notifier in &notifiers {
                        if let Err(e) = notifier.send(&format!("🚀 {} started.", snapshot.name), None).await {
                            eprintln!("Failed to send startup notification: {}", e);
                        }
                    }
                    first_run = false;
                }

                let now = Utc::now();
                let global_interval_secs = snapshot.interval_minutes * 60;
                let mut state_changed = false;

                for site in &snapshot.sites {
                    if site.paused { continue; }

                    // Calculate the specific interval for this site
                    // e.g. 300s / 5 freq = check every 60s
                    let site_interval_secs = global_interval_secs / site.frequency_multiplier.max(1);
                    
                    let last_check = site.last_check.unwrap_or_else(|| Utc::now() - chrono::Duration::days(1));
                    let elapsed = (now - last_check).num_seconds();

                    if elapsed >= site_interval_secs as i64 {
                        let (up, status) = checker::check_site(&client, &site.url, site.timeout_seconds).await;
                        stats::record_check(up);
                        
                        // Update failure counts and status in the master state
                        let site_copy = {
                            let mut lock = state.write().unwrap();
                            if let Some(s) = lock.sites.iter_mut().find(|s| s.url == site.url) {
                                s.last_status = Some(status.clone());
                                s.last_check = Some(Utc::now());
                                if up { s.consecutive_failures = 0; } else { s.consecutive_failures += 1; }
                                s.clone()
                            } else { site.clone() }
                        };
                        state_changed = true;

                        // Notification Logic
                        let display = format!("{} {} ({})", site_copy.emoji.clone().unwrap_or_default(), site_copy.name.clone().unwrap_or(site_copy.url.clone()), site_copy.url);
                        let is_silenced = snapshot.silence_until.map(|u| Utc::now() < u).unwrap_or(false);

                        let should_notify = if up {
                            tracker.update(&site_copy.url, &display, true).is_some()
                        } else {
                            if site_copy.consecutive_failures > snapshot.max_retries {
                                tracker.update(&site_copy.url, &display, false).is_some()
                            } else {
                                println!("Failure for {} suppressed (retry {}/{})", site_copy.url, site_copy.consecutive_failures, snapshot.max_retries);
                                false
                            }
                        };

                        if should_notify && !is_silenced {
                            if let Some(msg) = tracker.get_last_message(&site_copy.url) {
                                let targets = site_copy.recipients.as_deref();
                                for notifier in &notifiers {
                                    if let Err(e) = notifier.send(&msg, targets).await {
                                        eprintln!("Failed to send notification: {}", e);
                                    }
                                }
                            }
                        }

                        // Small gap between concurrent checks
                        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
                    }
                }

                if state_changed {
                    let lock = state.read().unwrap();
                    storage::save(&lock);
                }
            }
            _ = notify.notified() => {
                println!("Config change detected, re-syncing...");
            }
        }
    }
}
