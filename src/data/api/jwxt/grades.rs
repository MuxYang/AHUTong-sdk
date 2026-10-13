use crate::data::api::client::AHUClient;
use reqwest::Result;
use serde_json::Value;
const BASE_URL: &str = "https://jw.ahu.edu.cn";

impl AHUClient {
    pub async fn get_exam_info(&self) -> Result<String> {
        self.http
            .get(format!("{}/student/for-std/exam-arrange", BASE_URL))
            .send()
            .await?
            .text()
            .await
    }

    pub async fn get_grade_sheet_entry_url(&self) -> Result<String> {
        let resp = self
            .http
            .get(format!("{}/student/for-std/grade/sheet", BASE_URL))
            .send()
            .await?;
        Ok(resp.url().to_string())
    }

    pub async fn get_grade_info(&self, id: &str) -> Result<Value> {
        self.http
            .get(format!(
                "{}/student/for-std/grade/sheet/info/{}",
                BASE_URL, id
            ))
            .send()
            .await?
            .json::<Value>()
            .await
    }
}
