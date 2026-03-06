use async_trait::async_trait;
use anyhow::Result;

#[async_trait]
pub trait Notifier: Send + Sync {
    async fn send(&self, message: &str, target_recipients: Option<&[String]>) -> Result<()>;
}
