//! Compatibility reader for snapshots of the previous GuiXu string store.
//! The caller copies the original directory before invoking this module because
//! GuiXu opens writable maps even when its API is used only for reads.
use anyhow::{Context, Result, anyhow};
use guixu::{GuiXu, GuiXuError};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::path::Path;

mod index;
pub use index::list_keys;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ReadRequest {
    #[serde(rename = "box")]
    box_name: String,
    key: String,
}

#[derive(Serialize)]
struct ReadValue {
    #[serde(rename = "box")]
    box_name: String,
    key: String,
    value: Option<String>,
}

fn valid_name(value: &str) -> bool {
    !value.is_empty()
        && !value.contains(['/', '\\', ':'])
        && !value.contains("..")
        && !value.chars().any(char::is_control)
}

fn read(snapshot: &str, requests: &str) -> Result<Vec<ReadValue>> {
    if requests.len() > 1024 * 1024 {
        return Err(anyhow!("Migration request exceeds 1 MiB"));
    }
    let requests: Vec<ReadRequest> = serde_json::from_str(requests)?;
    if requests.len() > 4096 {
        return Err(anyhow!("Migration request exceeds 4096 keys"));
    }
    let path = Path::new(snapshot);
    if !path.is_dir() {
        return Err(anyhow!("Migration snapshot directory does not exist"));
    }
    for request in &requests {
        if !valid_name(&request.box_name) || !valid_name(&request.key) {
            return Err(anyhow!("Invalid migration box or key"));
        }
    }
    let database = GuiXu::new(path.to_path_buf()).context("Cannot open migration snapshot")?;
    let mut values = Vec::with_capacity(requests.len());
    for request in requests {
        // Missing legacy boxes stay absent; do not fabricate stores while reading.
        let value = if path.join(&request.box_name).is_file() {
            let mut kv = database.kv_box_for(&request.box_name)?;
            let value = match kv.get_string(&request.key) {
                Ok(value) => Some(value.to_string()),
                Err(GuiXuError::KeyNotFound(_)) => None,
                Err(error) => return Err(error).context("Cannot read migration key"),
            };
            kv.close()?;
            value
        } else {
            None
        };
        values.push(ReadValue {
            box_name: request.box_name,
            key: request.key,
            value,
        });
    }
    Ok(values)
}

pub fn read_strings(snapshot: &str, requests: &str) -> Value {
    match read(snapshot, requests) {
        Ok(values) => json!({"values": values}),
        Err(error) => json!({"error": error.to_string()}),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_legacy_binary_layout_and_missing_keys() {
        let root = std::env::temp_dir().join(format!("ahup-storage-test-{}", std::process::id()));
        std::fs::create_dir_all(&root).unwrap();
        let database = GuiXu::new(root.clone()).unwrap();
        let mut session = database.kv_box_for("session").unwrap();
        session
            .put_string("cookies_json", "legacy-cookie-data".to_string())
            .unwrap();
        session.close().unwrap();
        let result = read_strings(
            root.to_str().unwrap(),
            r#"[{"box":"session","key":"cookies_json"},{"box":"absent","key":"key"}]"#,
        );
        assert_eq!(result["values"][0]["value"], "legacy-cookie-data");
        assert!(result["values"][1]["value"].is_null());
        assert!(!root.join("absent").exists());
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn rejects_path_traversal_before_opening_a_box() {
        let result = read_strings(
            std::env::temp_dir().to_str().unwrap(),
            r#"[{"box":"../session","key":"cookies_json"}]"#,
        );
        assert!(result.get("error").is_some());
    }
}
