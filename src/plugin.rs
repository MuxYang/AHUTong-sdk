//! Stateless per-domain entry points. Cookies are explicit request/response data;
//! no process-global account, crawler, persistence or local HTTP server is started.
use crate::data::{api::client::AHUClient, crawler::Crawler};
use anyhow::{Result, anyhow};
use serde::Serialize;
use serde_json::{Value, json};
use std::sync::OnceLock;
use tokio::runtime::Runtime;

static RUNTIME: OnceLock<Result<Runtime, String>> = OnceLock::new();

fn runtime() -> Result<&'static Runtime> {
    RUNTIME
        .get_or_init(|| Runtime::new().map_err(|error| error.to_string()))
        .as_ref()
        .map_err(|error| anyhow!("Cannot start domain runtime: {error}"))
}

fn validate_cookies(cookies: &str) -> Result<()> {
    if cookies.len() > 1024 * 1024 {
        return Err(anyhow!("Session snapshot exceeds 1 MiB"));
    }
    // Legacy cookie_store snapshots are newline-delimited objects; Android also
    // provides the flat array format. Reject malformed input instead of silently
    // sending an anonymous request after a failed import.
    for value in serde_json::Deserializer::from_str(cookies).into_iter::<Value>() {
        let value = value?;
        if !value.is_object() && !value.is_array() {
            return Err(anyhow!(
                "Session snapshot must contain cookie objects or an array"
            ));
        }
    }
    Ok(())
}

fn envelope<T: Serialize>(result: Result<T>, cookies: String) -> Value {
    match result {
        Ok(value) => json!({"result": value, "cookies": cookies}),
        Err(error) => json!({"error": error.to_string(), "cookies": cookies}),
    }
}

fn request<T: Serialize>(cookies: &str, operation: impl FnOnce(&Crawler) -> Result<T>) -> Value {
    if let Err(error) = validate_cookies(cookies) {
        return envelope::<Value>(Err(error), cookies.to_string());
    }
    let client = AHUClient::new();
    client.load_cookies_json(cookies);
    let crawler = Crawler::new(client);
    let result = operation(&crawler);
    envelope(result, crawler.client.dump_cookies_json())
}

#[cfg(feature = "domain-account")]
pub fn login(username: &str, password: &str, cookies: &str) -> Value {
    request(cookies, |crawler| {
        if username.trim().is_empty() || password.is_empty() {
            return Err(anyhow!("Username and password are required"));
        }
        runtime()?.block_on(crawler.login(username, password))
    })
}

#[cfg(feature = "domain-schedule")]
pub fn get_schedule(cookies: &str) -> Value {
    request(cookies, |crawler| {
        runtime()?.block_on(crawler.get_schedule())
    })
}

#[cfg(feature = "domain-grades")]
pub fn get_grade(cookies: &str) -> Value {
    request(cookies, |crawler| {
        runtime()?.block_on(crawler.get_grade(None))
    })
}

#[cfg(feature = "domain-grades")]
pub fn get_exam_info(cookies: &str) -> Value {
    request(cookies, |crawler| {
        runtime()?.block_on(crawler.get_exam_info())
    })
}

#[cfg(feature = "domain-campus-card")]
fn card_request<T>(operation: impl std::future::Future<Output = Result<T>>) -> Result<T> {
    runtime()?.block_on(async {
        tokio::time::timeout(std::time::Duration::from_secs(12), operation)
            .await
            .map_err(|_| anyhow!("Campus card request timed out after 12 seconds"))?
    })
}

#[cfg(feature = "domain-campus-card")]
pub fn get_balance(cookies: &str) -> Value {
    request(cookies, |crawler| card_request(crawler.get_balance()))
}

#[cfg(feature = "domain-campus-card")]
pub fn get_qrcode(cookies: &str) -> Value {
    request(cookies, |crawler| card_request(crawler.get_qrcode()))
}

#[cfg(feature = "domain-campus-card")]
pub fn refresh_token(cookies: &str) -> Value {
    request(cookies, |crawler| {
        let manager = crate::data::auth::AuthManager::new(crawler.client.clone());
        card_request(manager.refresh_token())
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(feature = "domain-campus-card")]
    #[test]
    fn card_operation_has_a_total_deadline_even_if_the_protocol_never_completes() {
        let started = std::time::Instant::now();
        let result = card_request(std::future::pending::<Result<()>>());
        assert!(result.unwrap_err().to_string().contains("timed out"));
        assert!(started.elapsed() < std::time::Duration::from_secs(20));
    }

    #[test]
    fn invalid_sessions_do_not_execute_business_requests() {
        let result = request::<Value>("broken JSON", |_| panic!("must not send a request"));
        assert!(result.get("error").is_some());
        assert_eq!(result["cookies"], "broken JSON");
    }

    #[test]
    fn account_snapshots_are_isolated_between_calls() {
        let cookies = r#"[{"name":"SESSION","value":"first-user","domain":"jw.ahu.edu.cn","path":"/","secure":true}]"#;
        let first = request(cookies, |crawler| {
            Ok(crawler.client.get_cookies_flat_json())
        });
        let second = request("[]", |crawler| Ok(crawler.client.get_cookies_flat_json()));
        assert!(first["result"].as_str().unwrap().contains("first-user"));
        assert_eq!(second["result"], "[]");
    }

    #[test]
    fn native_cookie_snapshot_round_trips_without_cross_call_state() {
        let source = r#"[{"name":"SESSION","value":"native","domain":"jw.ahu.edu.cn","path":"/","secure":true}]"#;
        let first = request(source, |_| Ok(true));
        let restored = request(first["cookies"].as_str().unwrap(), |crawler| {
            Ok(crawler.client.get_cookies_flat_json())
        });
        assert!(restored["result"].as_str().unwrap().contains("native"));
    }
}
