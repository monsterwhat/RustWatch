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
                let current_config = state.whatsapp.clone();
                if self.whatsapp_notifier.is_none() || self.last_whatsapp_config.as_ref() != Some(&current_config) {
                    println!("Initializing WhatsApp notifier for {} recipients...", state.whatsapp.recipients.len());
                    match WhatsAppNotifier::new(state.whatsapp.recipients.clone(), self.state.clone(), self.notify.clone()).await {
                        Ok(w) => {
                            let name = {
                                let lock = self.state.read().unwrap();
                                lock.name.clone()
                            };
                            println!("WhatsApp notifier initialized.");
                            let _ = w.send(&format!("✅ {} successfully configured! (Remote commands active)", name), None).await;
                            
                            self.whatsapp_notifier = Some(Arc::new(w));
                            self.last_whatsapp_config = Some(current_config);
                        }
                        Err(e) => {
                            println!("❌ Failed to initialize WhatsApp: {}", e);
                            self.whatsapp_notifier = None;
                        }
                    }
                }
                if let Some(ref n) = self.whatsapp_notifier {
                    list.push(n.clone());
                }
            }
        } else {
            self.whatsapp_notifier = None;
            self.last_whatsapp_config = None;
        }

        list
    }
}
