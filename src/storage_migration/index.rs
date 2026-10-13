//! Read-only key enumeration for the pinned GuiXu format. Values are decoded by GuiXu.
use super::valid_name;
use anyhow::{Result, anyhow};
use serde_json::{Value, json};
use std::collections::BTreeMap;
use std::io::{Read, Seek, SeekFrom};
use std::path::Path;

fn list(snapshot: &str) -> Result<Vec<Value>> {
    let path = Path::new(snapshot);
    if !path.is_dir() {
        return Err(anyhow!("Migration snapshot directory does not exist"));
    }
    let mut entries = BTreeMap::new();
    for entry in std::fs::read_dir(path)? {
        let entry = entry?;
        let kind = entry.file_type()?;
        if !kind.is_file() || kind.is_symlink() {
            return Err(anyhow!(
                "Migration snapshot must contain only regular files"
            ));
        }
        let name = entry
            .file_name()
            .into_string()
            .map_err(|_| anyhow!("Invalid snapshot filename"))?;
        let Some(box_name) = name.strip_suffix("-K2IdMap") else {
            continue;
        };
        if !valid_name(box_name) || !path.join(box_name).is_file() {
            return Err(anyhow!("Invalid GuiXu box"));
        }
        let mut file = std::fs::File::open(entry.path())?;
        let size = file.metadata()?.len();
        if size < 12 {
            return Err(anyhow!("GuiXu key map is truncated"));
        }
        file.seek(SeekFrom::End(-8))?;
        let mut footer = [0u8; 8];
        file.read_exact(&mut footer)?;
        let logical = u64::from_be_bytes(footer);
        if logical < 4 || logical > size - 8 || logical > 32 * 1024 * 1024 {
            return Err(anyhow!("Invalid GuiXu key map length"));
        }
        file.seek(SeekFrom::Start(0))?;
        let mut bytes = vec![0u8; logical as usize];
        file.read_exact(&mut bytes)?;
        let mut cursor = 4usize;
        while cursor < bytes.len() {
            if bytes.len() - cursor < 13 {
                return Err(anyhow!("Truncated GuiXu key record"));
            }
            let length = i32::from_be_bytes(bytes[cursor..cursor + 4].try_into()?) as i64;
            let value_type = bytes[cursor + 4];
            cursor += 13;
            if length < 1 || length > 1024 * 1024 || length as usize > bytes.len() - cursor {
                return Err(anyhow!("Invalid GuiXu key length"));
            }
            let key = std::str::from_utf8(&bytes[cursor..cursor + length as usize])?;
            cursor += length as usize;
            if !valid_name(key) {
                return Err(anyhow!("Invalid GuiXu key name"));
            }
            if value_type != 0 && value_type != 7 {
                return Err(anyhow!(
                    "Unsupported non-string GuiXu value; migration was not committed"
                ));
            }
            entries.insert((box_name.to_string(), key.to_string()), value_type);
            if entries.len() > 100_000 {
                return Err(anyhow!("Too many GuiXu migration keys"));
            }
        }
    }
    Ok(entries
        .into_iter()
        .filter(|(_, kind)| *kind == 7)
        .map(|((box_name, key), _)| json!({"box": box_name, "key": key}))
        .collect())
}

pub fn list_keys(snapshot: &str) -> Value {
    match list(snapshot) {
        Ok(keys) => json!({"keys": keys}),
        Err(error) => json!({"error": error.to_string()}),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn indexes_live_string_keys_using_real_guixu_layout() {
        let root = std::env::temp_dir().join(format!("ahup-storage-index-{}", std::process::id()));
        let db = guixu::GuiXu::new(root.clone()).unwrap();
        let mut session = db.kv_box_for("session").unwrap();
        session
            .put_string("cookies_json", "fixture".to_string())
            .unwrap();
        session.put_string("removed", "unused".to_string()).unwrap();
        session.remove("removed").unwrap();
        session.close().unwrap();
        let listed = list(root.to_str().unwrap()).unwrap();
        assert_eq!(listed, vec![json!({"box":"session","key":"cookies_json"})]);
        std::fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn corrupt_map_is_an_error_not_an_empty_migration() {
        let root =
            std::env::temp_dir().join(format!("ahup-storage-corrupt-{}", std::process::id()));
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(root.join("session"), [0u8, 1]).unwrap();
        std::fs::write(root.join("session-K2IdMap"), [0u8; 12]).unwrap();
        assert!(list(root.to_str().unwrap()).is_err());
        std::fs::remove_dir_all(root).unwrap();
    }
}
