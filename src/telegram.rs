use crate::notifier::Notifier;
use async_trait::async_trait;
use anyhow::Result;
use reqwest::Client;

pub struct TelegramNotifier {
    client: Client,
    bot_token: String,
    chat_id: String,
}

impl TelegramNotifier {
    pub fn new(client: Client, bot_token: String, chat_id: String) -> Self {
        Self { client, bot_token, chat_id }
    }
}

#[async_trait]
impl Notifier for TelegramNotifier {
    async fn send(&self, message: &str, _target_recipients: Option<&[String]>) -> Result<()> {
        let url = format!(
            "https://api.telegram.org/bot{}/sendMessage",
            self.bot_token
        );

        self.client
            .post(url)
            .json(&serde_json::json!({
                "chat_id": self.chat_id,
                "text": message
            }))
            .send()
            .await?;

        Ok(())
    }
}
