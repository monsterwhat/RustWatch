use crate::{
    app_state::{AppState, TelegramConfig, WhatsAppConfig},
    notifier::Notifier,
    telegram::TelegramNotifier,
    whatsapp::WhatsAppNotifier,
};
use std::sync::{Arc, RwLock};
use reqwest::Client;
use tokio::sync::Notify;

pub struct NotifierManager {
    client: Client,
    telegram_notifier: Option<Arc<TelegramNotifier>>,
    whatsapp_notifier: Option<Arc<WhatsAppNotifier>>,
    last_telegram_config: Option<TelegramConfig>,
    last_whatsapp_config: Option<WhatsAppConfig>,
    state: Arc<RwLock<AppState>>,
    notify: Arc<Notify>,
    whatsapp_started: bool,
}

impl NotifierManager {
    pub fn new(state: Arc<RwLock<AppState>>, notify: Arc<Notify>) -> Self {
        Self {
            client: Client::new(),
            telegram_notifier: None,
            whatsapp_notifier: None,
            last_telegram_config: None,
            last_whatsapp_config: None,
            state,
            notify,
            whatsapp_started: false,
        }
    }

    pub async fn get_notifiers(
        &mut self,
        state: &AppState,
    ) -> Vec<Arc<dyn Notifier>> {
        let mut list: Vec<Arc<dyn Notifier>> = vec![];

        // Telegram
        if state.telegram.enabled {
            if let (Some(token), Some(chat_id)) =
                (&state.telegram.bot_token, &state.telegram.chat_id)
            {
                let current_config = state.telegram.clone();
                if self.telegram_notifier.is_none() || self.last_telegram_config.as_ref() != Some(&current_config) {
                    self.telegram_notifier = Some(Arc::new(TelegramNotifier::new(
                        self.client.clone(),
                        token.clone(),
                        chat_id.clone(),
                    )));
                    self.last_telegram_config = Some(current_config);
                }
                if let Some(ref n) = self.telegram_notifier {
                    list.push(n.clone());
                }
            }
        } else {
            self.telegram_notifier = None;
            self.last_telegram_config = None;
        }

        // WhatsApp
        if state.whatsapp.enabled {
            if !state.whatsapp.recipients.is_empty() {
                // Only create once - don't recreate on every tick
                if self.whatsapp_notifier.is_none() {
                    println!("Initializing WhatsApp notifier for {} recipients...", state.whatsapp.recipients.len());
                    match WhatsAppNotifier::new(state.whatsapp.recipients.clone(), self.state.clone(), self.notify.clone()).await {
                        Ok(w) => {
                            self.whatsapp_notifier = Some(Arc::new(w));
                            self.last_whatsapp_config = Some(state.whatsapp.clone());
                        }
                        Err(e) => {
                            println!("❌ Failed to initialize WhatsApp: {}", e);
                            self.whatsapp_notifier = None;
                        }
                    }
                }
                
                // Add to list only if connected, and send startup message once
                if let Some(ref n) = self.whatsapp_notifier {
                    if n.is_connected() {
                        if !self.whatsapp_started {
                            let name = {
                                let lock = self.state.read().unwrap();
                                lock.name.clone()
                            };
                            println!("WhatsApp notifier initialized.");
                                                        if let Err(e) = n.send_startup_message(&name).await {
                                eprintln!("Failed to send startup message: {}", e);
                            }
                            self.whatsapp_started = true;
                        }
                        list.push(n.clone());
                    }
                }
            }
        } else {
            self.whatsapp_notifier = None;
            self.last_whatsapp_config = None;
        }

        list
    }
}
