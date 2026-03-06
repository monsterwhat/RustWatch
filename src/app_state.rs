use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

#[derive(Clone, Serialize, Deserialize)]
pub struct Site {
    pub url: String,
    pub name: Option<String>,
    pub emoji: Option<String>,
    pub timeout_seconds: u64,
    pub recipients: Option<Vec<String>>,
    pub last_status: Option<String>,
    pub last_check: Option<DateTime<Utc>>,
    #[serde(default)]
    pub paused: bool,
    #[serde(default = "default_frequency")]
    pub frequency_multiplier: u64,
    #[serde(default)]
    pub consecutive_failures: u64,
}

fn default_frequency() -> u64 { 1 }

#[derive(Clone, Serialize, Deserialize, PartialEq)]
pub struct TelegramConfig {
    pub enabled: bool,
    pub bot_token: Option<String>,
    pub chat_id: Option<String>,
}

#[derive(Clone, Serialize, Deserialize, PartialEq)]
pub struct WhatsAppConfig {
    pub enabled: bool,
    pub recipients: Vec<String>,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct AppState {
    pub name: String,
    pub interval_minutes: u64,
    pub sites: Vec<Site>,
    pub telegram: TelegramConfig,
    pub whatsapp: WhatsAppConfig,
    pub silence_until: Option<DateTime<Utc>>,
    #[serde(default = "default_max_retries")]
    pub max_retries: u64,
}

fn default_max_retries() -> u64 { 0 }

impl Default for AppState {
    fn default() -> Self {
        Self {
            name: "Monitor Daemon".to_string(),
            interval_minutes: 5,
            sites: vec![],
            telegram: TelegramConfig {
                enabled: false,
                bot_token: None,
                chat_id: None,
            },
            whatsapp: WhatsAppConfig {
                enabled: false,
                recipients: vec![],
            },
            silence_until: None,
            max_retries: 0,
        }
    }
}
