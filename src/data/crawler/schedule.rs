use super::Crawler;
use crate::data::model::Course;
use crate::utils::parser::Parser;
use anyhow::{Result, anyhow};
use log::{debug, error, info, warn};

impl Crawler {
    pub async fn get_schedule(&self) -> Result<Vec<Course>> {
        info!("[RustSDKSchedule] Starting get_schedule...");
        self.client.log_current_cookies();

        let basic_info_html = self.client.fetch_course_table_basic_info().await?;
        info!(
            "[RustSDKSchedule] Fetched basic info HTML. Size: {}",
            basic_info_html.len()
        );

        // Check if it's a login page
        if basic_info_html.contains("<title>登入页面</title>")
            || basic_info_html.contains("id=\"loginForm\"")
        {
            error!(
                "[RustSDKSchedule] Detected login page instead of course table. Session expired or invalid."
            );
            return Err(anyhow!(
                "Session expired or invalid. Redirected to login page."
            ));
        }

        let (semester_id, semester_name) = Parser::parse_current_semester(&basic_info_html)
            .ok_or_else(|| {
                error!("[RustSDKSchedule] Failed to parse current semester ID");
                anyhow!("Failed to parse current semester ID")
            })?;

        info!(
            "[RustSDKSchedule] Parsed semester: id={}, name={}",
            semester_id, semester_name
        );

        let course_json = self
            .client
            .get_course(semester_id, semester_id, false)
            .await?;
        debug!("[RustSDKSchedule] Course JSON: {:?}", course_json);

        // 检查返回的 JSON 中是否有错误提示
        if let Some(err_msg) = course_json.get("message").and_then(|v| v.as_str()) {
            warn!("[RustSDKSchedule] Server returned message: {}", err_msg);
        }

        let mut courses = Vec::new();

        let activities_opt = course_json
            .get("studentTableVms")
            .and_then(|v| v.get(0))
            .and_then(|v| v.get("activities"))
            .and_then(|v| v.as_array());

        // 如果 activities 为空，可能是因为 Master 分支使用了不同的学期 ID
        // 尝试检查是否获取到了正确的学期 ID。如果 semester_id 与当前实际学期不符，可能导致查不到课。
        // Master 分支代码：val courseTable = JwxtApi.API.getCourse(currentSemesterJson.id, currentSemesterJson.id)
        // SDK 代码：self.client.get_course(semester_id, semester_id, false)
        // 参数一致。

        if activities_opt.is_none() {
            error!(
                "[RustSDKSchedule] 'studentTableVms[0].activities' not found or not an array. JSON keys: {:?}",
                course_json
                    .as_object()
                    .map(|o| o.keys().collect::<Vec<_>>())
            );
            // 这里不直接报错，而是允许返回空列表，因为有些时候确实没课
            // return Err(anyhow!("Invalid course table JSON structure"));
        }

        if let Some(activities) = activities_opt {
            info!("[RustSDKSchedule] Found {} activities", activities.len());

            for (idx, act) in activities.iter().enumerate() {
                let mut week_indexes: Vec<i32> =
                    serde_json::from_value(act["weekIndexes"].clone()).unwrap_or_default();
                week_indexes.sort_unstable();

                if week_indexes.is_empty() {
                    warn!(
                        "[RustSDKSchedule] Activity {} has empty weekIndexes, skipping.",
                        idx
                    );
                    continue;
                }

                let start_week = *week_indexes.first().unwrap();
                let end_week = *week_indexes.last().unwrap();

                let start_unit = act["startUnit"].as_i64().unwrap_or(1);
                let end_unit = act["endUnit"].as_i64().unwrap_or(1);
                let length = end_unit - start_unit + 1;

                let teacher = if let Some(arr) = act["teacherNames"].as_array() {
                    arr.iter()
                        .map(|v| v.as_str().unwrap_or(""))
                        .collect::<Vec<_>>()
                        .join(",")
                } else {
                    act["teacherNames"].as_str().unwrap_or("").to_string()
                };

                let weekday_str = if let Some(n) = act["weekday"].as_i64() {
                    n.to_string()
                } else if let Some(s) = act["weekday"].as_str() {
                    s.to_string()
                } else {
                    act["weekday"].to_string() // Fallback, though risky if it's null or object
                };

                let lesson_id_str = if let Some(n) = act["lessonId"].as_i64() {
                    n.to_string()
                } else if let Some(s) = act["lessonId"].as_str() {
                    s.to_string()
                } else {
                    act["lessonId"].to_string()
                };

                let course = Course {
                    name: act["courseName"].as_str().unwrap_or("").to_string(),
                    teacher,
                    location: act["room"].as_str().unwrap_or("未知").to_string(),
                    week_indexes,
                    start_week: start_week.to_string(),
                    end_week: end_week.to_string(),
                    start_time: start_unit.to_string(),
                    length: length.to_string(),
                    weekday: weekday_str,
                    course_id: lesson_id_str,
                };
                // info!("[RustSDKSchedule] Parsed course: {}", course.name);
                courses.push(course);
            }
        } else {
            warn!("[RustSDKSchedule] No activities found in response.");
        }

        info!(
            "[RustSDKSchedule] Successfully parsed {} courses.",
            courses.len()
        );
        Ok(courses)
    }
}
