use super::Crawler;
use crate::data::model::Exam;
use anyhow::{Context, Result, anyhow};
use serde_json::Value;

impl Crawler {
    pub async fn get_exam_info(&self) -> Result<Vec<Exam>> {
        let html = self.client.get_exam_info().await?;
        let re = regex::Regex::new(r"(?s)studentExamInfoVms\s*=\s*(\[.*?]);").unwrap();

        let json_str = re
            .captures(&html)
            .and_then(|caps| caps.get(1))
            .map(|m| m.as_str())
            .ok_or_else(|| anyhow!("Failed to extract studentExamInfoVms from HTML"))?;

        let fixed_json = json_str.replace("'", "\"");

        let items: Vec<Value> =
            serde_json::from_str(&fixed_json).context("Failed to parse extracted JSON")?;

        let mut exams = Vec::new();
        for item in items {
            let course_name = item["course"]["nameZh"].as_str().unwrap_or("");
            let exam_type = item["examType"]["nameZh"].as_str().unwrap_or("");
            let course_display = format!("{}({})", course_name, exam_type);

            let time = item["examTime"].as_str().unwrap_or("").to_string();
            let seat_num = item["seatNo"].to_string();

            let campus = item["requiredCampus"]["nameZh"].as_str().unwrap_or("");
            let room = item["room"].as_str().unwrap_or("");
            let location = format!("{}-{}", campus, room);

            let finished = item["finished"].as_bool().unwrap_or(false);

            exams.push(Exam {
                course: course_display,
                time,
                seat_num,
                location,
                finished,
            });
        }
        Ok(exams)
    }

    pub async fn get_grade(&self, student_id: Option<String>) -> Result<Value> {
        let id = match student_id {
            Some(id) => id,
            None => {
                // 自动获取 ID
                let url = self.client.get_grade_sheet_entry_url().await?;
                let parts: Vec<&str> = url.split('/').collect();
                parts
                    .last()
                    .map(|s| s.to_string())
                    .filter(|s| !s.is_empty())
                    .ok_or_else(|| anyhow!("Failed to extract Student ID from URL: {}", url))?
            }
        };

        let grade_json = self.client.get_grade_info(&id).await?;
        Ok(grade_json)
    }
}
