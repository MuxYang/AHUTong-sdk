use crate::data::api::client::AHUClient;
use reqwest::Response;
use reqwest::Result;
const BASE_URL: &str = "https://jw.ahu.edu.cn";

impl AHUClient {
    pub async fn fetch_login_info(&self) -> Result<(String, reqwest::Url)> {
        let resp = self
            .http
            .get(format!("{}/student/sso/login", BASE_URL))
            .send()
            .await?;

        let url = resp.url().clone();
        let text = resp.text().await?;

        Ok((text, url))
    }

    pub async fn get_grade_sheet_entry(&self) -> Result<String> {
        self.http
            .get(format!("{}/student/for-std/grade/sheet", BASE_URL))
            .send()
            .await?
            .text()
            .await
    }

    pub async fn device_login(
        &self,
        url: &str,
        username_len: usize,
        password_len: usize,
        rsa: &str,
    ) -> Result<String> {
        let params = [
            ("ul", username_len.to_string()),
            ("pl", password_len.to_string()),
            ("rsa", rsa.to_string()),
            ("method", "login".to_string()),
        ];

        self.http.post(url).form(&params).send().await?.text().await
    }

    pub async fn jwxt_login(
        &self,
        url: &str,
        rsa: &str,
        username_len: usize,
        password_len: usize,
        lt: &str,
        execution: &str,
        event_id: &str,
    ) -> Result<Response> {
        let params = [
            ("rsa", rsa.to_string()),
            ("ul", username_len.to_string()),
            ("pl", password_len.to_string()),
            ("lt", lt.to_string()),
            ("execution", execution.to_string()),
            ("_eventId", event_id.to_string()),
        ];

        self.http.post(url).form(&params).send().await
    }
}
