use super::Crawler;
use crate::data::model::User;
use crate::utils::{des::DES, parser::Parser};
use anyhow::{Context, Result, anyhow};
use log::{debug, error, info, warn};
use serde_json::Value;
const JWXT_HOME: &str = "https://jw.ahu.edu.cn/student/home";

impl Crawler {
    pub async fn login(&self, username: &str, password: &str) -> Result<User> {
        info!("Starting login flow for user: {}", username);
        let mut login_success_info: Option<Value> = None;
        let mut last_auth_code_error: Option<String> = None;
        for i in 0..5 {
            info!("ADWMH login attempt {}/5", i + 1);
            let auth_code_bytes = match self.client.get_auth_code().await {
                Ok(bytes) => bytes,
                Err(e) => {
                    let message = format!("{:#}", e);
                    warn!("Failed to get auth code on attempt {}: {}", i + 1, message);
                    last_auth_code_error = Some(message);
                    continue;
                }
            };
            debug!("Auth code bytes length: {}", auth_code_bytes.len());

            let captcha_res = self
                .client
                .get_captcha_result("https://118.25.8.226/ocr/captcha", auth_code_bytes.to_vec())
                .await;

            let captcha_code = match captcha_res {
                Ok(res) => {
                    debug!("OCR Service Response: {:?}", res);
                    res["result"].as_str().unwrap_or("").to_string()
                }
                Err(e) => {
                    warn!("OCR Service Request Failed: {:?}", e);
                    String::new()
                }
            };

            if captcha_code.is_empty() {
                warn!("Captcha recognition failed (empty result), retrying...");
                continue;
            }
            debug!("Captcha recognized: {}", captcha_code);

            let adwmh_login = self
                .client
                .login_with_captcha(username, password, 0, &captcha_code)
                .await?;
            if adwmh_login["code"].as_i64() == Some(10000) {
                info!("ADWMH login successful");
                login_success_info = Some(adwmh_login);
                break;
            } else {
                warn!("ADWMH login failed: {:?}", adwmh_login);
            }
        }

        // Master 分支逻辑：ADWMH 登录必须成功，否则整个登录失败
        let login_info = login_success_info.ok_or_else(|| {
            if let Some(error) = last_auth_code_error {
                anyhow!("Failed to get auth code after 5 attempts: {}", error)
            } else {
                anyhow!("ADWMH login failed after 5 attempts")
            }
        })?;

        let user_obj = &login_info["object"]["user"];
        let user = User {
            username: user_obj["userName"].as_str().unwrap_or("").to_string(),
            id_number: user_obj["idNumber"].as_str().unwrap_or("").to_string(),
        };

        info!("Fetching CAS login page...");
        let (login_page_html, current_url) = self.client.fetch_login_info().await?;

        // 检查是否已经登录 (重定向到了首页)
        let current_url_str = current_url.as_str();
        if current_url_str.ends_with("/student/home") || current_url_str == JWXT_HOME {
            info!("Already logged in (redirected to home). Skipping CAS login.");
            let verify = self.client.get_grade_sheet_entry().await;
            match verify {
                Ok(_) => {
                    info!("Session verification successful (Grade sheet accessible).");
                    // 打印 Cookie
                    debug!("Cookies: {}", self.client.get_cookies_flat_json());
                    return Ok(user);
                }
                Err(e) => {
                    warn!(
                        "Session verification failed despite being on home page: {}. Forcing re-login...",
                        e
                    );
                    // 如果验证失败，说明 Cookie 其实是坏的，或者被软拦截了。
                    // 这里可以选择继续往下走（尝试 CAS 登录），或者报错。
                    // 继续往下走可能会因为没有 LT 参数而失败。
                    // 最好的办法是：清除 Cookie，重新开始？
                    // 暂时先让它继续往下走，看看 Parser 能不能解析出东西（大概率不能，因为在 Home 页）。
                }
            }
        }

        let (lt, execution, action) = match Parser::parse_cas_params(&login_page_html) {
            Some(params) => params,
            None => {
                // 如果解析失败，可能是因为 ADWMH 登录后 Cookie 已过期，CAS 页面不是预期的登录页
                // 或者是因为已经被重定向到了其他页面
                error!("CAS param parse failed. Current URL: {}", current_url_str);
                // 打印一部分 HTML 以供调试，但截断以防止日志过长
                let preview_len = std::cmp::min(login_page_html.len(), 500);
                error!(
                    "HTML content (first {} chars): {:.500}",
                    preview_len, login_page_html
                );
                return Err(anyhow!(
                    "Failed to parse CAS login params from {}",
                    current_url_str
                ));
            }
        };

        info!(
            "CAS params extracted: lt={}, execution={}, action={}",
            lt, execution, action
        );

        // Kotlin 使用 length() (UTF-16 char count)，Rust len() 是 byte count。
        // 为了与 Java 行为完全一致（包括 Emoji 等 surrogate pairs），必须使用 UTF-16 code units count。
        let ul = username.encode_utf16().count();
        let pl = password.encode_utf16().count();

        let cipher = DES::str_enc(&format!("{}{}{}", username, password, lt), "1", "2", "3");

        info!("Performing device login...");
        self.client
            .device_login("https://one.ahu.edu.cn/cas/device", ul, pl, &cipher)
            .await
            .context("Device login failed")?;

        let login_url = if action.starts_with("http") {
            action
        } else {
            format!("https://one.ahu.edu.cn{}", action)
        };

        info!("Performing JWXT CAS login to: {}", login_url);
        let response = self
            .client
            .jwxt_login(&login_url, &cipher, ul, pl, &lt, &execution, "submit")
            .await
            .context("CAS login request failed")?;

        // if (jwxtResponse.raw().request.url.toString().endsWith(Constants.JWXT_HOME))
        let final_url = response.url().as_str();
        info!("JWXT login final URL: {}", final_url);

        if final_url == JWXT_HOME || final_url.ends_with("/student/home") {
            info!("JWXT login success verified");
            Ok(user)
        } else {
            warn!("JWXT login failed (redirect mismatch). Expected ending with /student/home");
            Err(anyhow!("JWXT Login failed: redirected to {}", final_url))
        }
    }
}
