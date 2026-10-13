use super::Parser;
use crate::data::model::Exam;
use regex::Regex;
use serde::Deserialize;
use std::sync::OnceLock;

impl Parser {
    pub fn parse_exam_info(html: &str) -> Vec<Exam> {
        static RE: OnceLock<Regex> = OnceLock::new();
        let re = RE.get_or_init(|| Regex::new(r"studentExamInfoVms\s*=\s*(\[.*?]);").unwrap());

        let mut exams = Vec::new();

        if let Some(caps) = re.captures(html) {
            if let Some(raw_json_match) = caps.get(1) {
                let fixed_json = raw_json_match.as_str().replace("'", "\"");

                #[derive(Deserialize)]
                struct RawExam {
                    #[serde(rename = "course")]
                    course: RawExamCourse,
                    #[serde(rename = "examTime")]
                    exam_time: String,
                    #[serde(rename = "seatNo")]
                    seat_no: i32,
                    #[serde(rename = "room")]
                    room: String,
                    #[serde(rename = "requiredCampus")]
                    campus: RawExamCampus,
                    finished: bool,
                }
                #[derive(Deserialize)]
                struct RawExamCourse {
                    #[serde(rename = "nameZh")]
                    name_zh: String,
                }
                #[derive(Deserialize)]
                struct RawExamCampus {
                    #[serde(rename = "nameZh")]
                    name_zh: String,
                }

                if let Ok(raw_list) = serde_json::from_str::<Vec<RawExam>>(&fixed_json) {
                    for item in raw_list {
                        exams.push(Exam {
                            course: item.course.name_zh,
                            time: item.exam_time,
                            seat_num: item.seat_no.to_string(),
                            location: format!("{}-{}", item.campus.name_zh, item.room),
                            finished: item.finished,
                        });
                    }
                }
            }
        }
        exams
    }
}
