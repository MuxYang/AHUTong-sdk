use super::Crawler;
use anyhow::Result;
use serde_json::Value;

impl Crawler {
    pub async fn get_qrcode(&self) -> Result<Value> {
        Ok(self.client.get_qrcode().await?)
    }

    pub async fn get_balance(&self) -> Result<Value> {
        Ok(self.client.get_balance().await?)
    }
}
