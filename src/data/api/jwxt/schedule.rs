use crate::data::api::client::AHUClient;
use reqwest::Result;
use serde_json::Value;
const BASE_URL: &str = "https://jw.ahu.edu.cn";

impl AHUClient {
    pub async fn get_course(
        &self,
        semester_path_id: i32,
        _semester_query_id: i32,
        has_experiment: bool,
    ) -> Result<Value> {
        let url = format!(
            "{}/student/for-std/course-table/semester/{}/print-data",
            BASE_URL, semester_path_id
        );
        self.http
            .get(url)
            .query(&[
                ("semesterId", semester_path_id.to_string()),
                ("hasExperiment", has_experiment.to_string()),
            ])
            .send()
            .await?
            .json::<Value>()
            .await
    }

    pub async fn fetch_course_table_basic_info(&self) -> Result<String> {
        self.http
            .get(format!("{}/student/for-std/course-table", BASE_URL))
            .send()
            .await?
            .text()
            .await
    }

    pub async fn get_current_teach_week(&self) -> Result<Value> {
        self.http
            .get(format!("{}/student/home/get-current-teach-week", BASE_URL))
            .send()
            .await?
            .json::<Value>()
            .await
    }
}
