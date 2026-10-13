use crate::data::api::client::AHUClient;
use reqwest::Result;
use serde_json::Value;
const BASE_URL: &str = "https://adwmh.ahu.edu.cn";

impl AHUClient {
    pub async fn get_balance(&self) -> Result<Value> {
        self.http
            .get(format!("{}/xzxcard/yue", BASE_URL))
            .send()
            .await?
            .json::<Value>()
            .await
    }

    pub async fn get_qrcode(&self) -> Result<Value> {
        self.http
            .get(format!("{}/xzxcard/qrcode", BASE_URL))
            .send()
            .await?
            .json::<Value>()
            .await
    }
}
