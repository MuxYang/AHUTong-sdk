use super::Parser;
use regex::Regex;
use std::sync::OnceLock;

impl Parser {
    pub fn parse_cas_params(html: &str) -> Option<(String, String, String)> {
        static INPUT_RE: OnceLock<Regex> = OnceLock::new();
        let input_re = INPUT_RE.get_or_init(|| Regex::new(r"<input([^>]+)>").unwrap());

        static NAME_RE: OnceLock<Regex> = OnceLock::new();
        let name_re = NAME_RE.get_or_init(|| Regex::new(r#"name="([^"]+)""#).unwrap());

        static VALUE_RE: OnceLock<Regex> = OnceLock::new();
        let value_re = VALUE_RE.get_or_init(|| Regex::new(r#"value="([^"]*)""#).unwrap());

        static ACTION_RE: OnceLock<Regex> = OnceLock::new();
        // 修正 action 提取正则，处理 action 属性在不同位置或换行的情况
        let action_re =
            ACTION_RE.get_or_init(|| Regex::new(r#"<form[\s\S]*?action="([^"]+)""#).unwrap());

        let mut lt = String::new();
        let mut execution = String::new();

        for cap in input_re.captures_iter(html) {
            let attrs = &cap[1];
            if let Some(name_cap) = name_re.captures(attrs) {
                let name = &name_cap[1];
                if name == "lt" {
                    if let Some(val_cap) = value_re.captures(attrs) {
                        lt = val_cap[1].to_string();
                    }
                } else if name == "execution" {
                    if let Some(val_cap) = value_re.captures(attrs) {
                        execution = val_cap[1].to_string();
                    }
                }
            }
        }

        let mut action = action_re
            .captures(html)
            .map(|c| c[1].to_string())
            .unwrap_or_default();

        // Simple HTML entity decode for URL
        action = action.replace("&amp;", "&");

        // 增加容错：如果正则提取失败，打印一下 HTML 片段帮助排查（生产环境建议移除）
        if lt.is_empty() || execution.is_empty() {
            // 这里我们无法打印日志，因为 parser.rs 没有引入 log crate。
            // 考虑返回 None，由调用者记录。
            // 但如果 lt 为空，可能是因为正则匹配问题，我们尝试更宽松的正则？
            // 目前先保持原样，调用者已经打印了 "Failed to parse CAS login params"
        }

        if !lt.is_empty() && !execution.is_empty() {
            Some((lt, execution, action))
        } else {
            None
        }
    }
}
