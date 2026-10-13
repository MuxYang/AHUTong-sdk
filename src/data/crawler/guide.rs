use super::Crawler;
use log::{error, info, warn};

impl Crawler {
    pub async fn download_calendar(&self, save_path: &str) -> anyhow::Result<()> {
        let url = "https://openahu.org/download/xiaoli.jpg";

        info!("Crawler: Start downloading calendar.");
        info!("Crawler: Target URL: {}", url);
        info!("Crawler: Local Save Path: {}", save_path);

        // --- 第一步：建立网络连接 ---
        info!("Crawler: Sending HTTP GET request...");
        let response = match self.client.http.get(url).send().await {
            Ok(resp) => {
                info!("Crawler: Connection established.");
                resp
            }
            Err(e) => {
                error!("Crawler: Network Request FAILED. Could not connect to server.");
                error!("Crawler: Network Error Details: {:?}", e);
                if e.is_timeout() {
                    error!("Crawler: Reason: Connection Timed Out.");
                } else if e.is_connect() {
                    error!("Crawler: Reason: Connection Refused/DNS Error.");
                }
                return Err(e.into());
            }
        };

        // --- 第二步：检查 HTTP 状态码 ---
        let status = response.status();
        info!("Crawler: Server returned HTTP Status: {}", status);

        if !status.is_success() {
            error!("Crawler: Server refused the request!");
            // 尝试读取服务器返回的错误信息（如果有）
            if let Ok(text) = response.text().await {
                error!("Crawler: Server Error Body: {}", text);
            }
            return Err(anyhow::anyhow!(
                "HTTP Request failed with status: {}",
                status
            ));
        }

        // --- 第三步：下载数据 ---
        info!("Crawler: Reading response body bytes...");
        let content_length = response.content_length().unwrap_or(0);
        info!("Crawler: Expected content length: {} bytes", content_length);

        let bytes = match response.bytes().await {
            Ok(b) => {
                info!("Crawler: Successfully downloaded {} bytes.", b.len());
                b
            }
            Err(e) => {
                error!("Crawler: Failed to read body bytes from stream.");
                error!("Crawler: Stream Error Details: {:?}", e);
                return Err(e.into());
            }
        };

        // --- 第四步：文件写入 ---
        // 检查父目录是否存在，这在 Android 上很重要
        let path_obj = std::path::Path::new(save_path);
        if let Some(parent) = path_obj.parent() {
            if !parent.exists() {
                info!(
                    "Crawler: Parent directory {:?} does not exist. Attempting to create...",
                    parent
                );
                if let Err(e) = tokio::fs::create_dir_all(parent).await {
                    error!("Crawler: Failed to create parent directory: {:?}", e);
                    // 这里不return，尝试直接写试试，或者直接报错
                }
            }
        }

        info!("Crawler: Writing bytes to file system...");
        match tokio::fs::write(save_path, &bytes).await {
            Ok(_) => {
                info!("Crawler: File write operation returned OK.");
                // 二次确认文件是否存在
                if path_obj.exists() {
                    info!(
                        "Crawler: VERIFICATION SUCCESS: File exists at {}",
                        save_path
                    );
                } else {
                    warn!("Crawler: VERIFICATION WARNING: Write returned OK but file not found!");
                }
                Ok(())
            }
            Err(e) => {
                error!("Crawler: File Write FAILED.");
                error!("Crawler: IO Error Details: {:?}", e);
                // 常见的 IO 错误分析
                match e.kind() {
                    std::io::ErrorKind::PermissionDenied => error!(
                        "Crawler: Reason: Permission Denied. Check Android Manifest/Storage Scopes."
                    ),
                    std::io::ErrorKind::NotFound => error!("Crawler: Reason: Directory Not Found."),
                    _ => error!("Crawler: Reason: Other IO Error."),
                }
                Err(e.into())
            }
        }
    }
}
