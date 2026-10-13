#[cfg(feature = "domain-account")]
#[test]
fn account_parser_preserves_cas_fields_and_entity_decoding() {
    let html = r#"<form action="/cas/login?service=one&amp;source=app"><input name="lt" value="LT-1"><input name="execution" value="e1s1"></form>"#;
    let parsed = ahutong_rs::utils::parser::Parser::parse_cas_params(html).unwrap();
    assert_eq!(
        parsed,
        (
            "LT-1".to_string(),
            "e1s1".to_string(),
            "/cas/login?service=one&source=app".to_string()
        )
    );
}

#[cfg(feature = "domain-account")]
#[test]
fn incomplete_cas_page_is_not_accepted_as_authenticated() {
    assert!(
        ahutong_rs::utils::parser::Parser::parse_cas_params("<html>请重新登录</html>").is_none()
    );
}

#[cfg(feature = "domain-schedule")]
#[test]
fn schedule_parser_keeps_nested_semester_data() {
    let html = r#"<script>var semesters = []; var currentSemester = {"id":318,"calendarAssoc":{"id":1},"name":"2026-2027-1"};</script>"#;
    assert_eq!(
        ahutong_rs::utils::parser::Parser::parse_current_semester(html),
        Some((318, "2026-2027-1".to_string()))
    );
}

#[cfg(feature = "domain-schedule")]
#[test]
fn long_chinese_undergraduate_and_postgraduate_pages_keep_utf8_boundaries() {
    for (id, label) in [(318, "本科教学学期"), (319, "研究生培养学期")] {
        let banner = "安徽大学课表提示".repeat(200);
        let html = format!(
            r#"<script>/*{banner}*/var semesters=[]; var currentSemester={{"id":{id},"name":"{label}"}};</script>"#
        );
        assert_eq!(
            ahutong_rs::utils::parser::Parser::parse_current_semester(&html),
            Some((id, label.to_string()))
        );
    }
}

#[cfg(feature = "domain-schedule")]
#[test]
fn malformed_long_chinese_page_returns_none_without_aborting_the_process() {
    let html = format!(
        "<script>var semesters=[];var currentSemester = /*{}*/;</script>",
        "学期参数暂时不可用".repeat(250)
    );
    assert_eq!(
        ahutong_rs::utils::parser::Parser::parse_current_semester(&html),
        None
    );
}

#[cfg(feature = "domain-grades")]
#[test]
fn exams_keep_legacy_field_mapping() {
    let html = r#"studentExamInfoVms = [{"course":{"nameZh":"离散数学"},"examTime":"2026-12-20","seatNo":12,"room":"A101","requiredCampus":{"nameZh":"磬苑"},"finished":false}];"#;
    let exams = ahutong_rs::utils::parser::Parser::parse_exam_info(html);
    assert_eq!(exams.len(), 1);
    assert_eq!(exams[0].course, "离散数学");
    assert_eq!(exams[0].seat_num, "12");
    assert_eq!(exams[0].location, "磬苑-A101");
}
