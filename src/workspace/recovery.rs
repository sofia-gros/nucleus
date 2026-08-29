/// ワークスペースのクラッシュリカバリおよび未保存バッファ自動スナップショット管理

use std::path::{Path, PathBuf};
use std::fs;

/// リカバリ対象のスナップショット情報
#[derive(Clone, Debug, PartialEq)]
pub struct RecoverySnapshot {
    pub original_path: String,
    pub backup_file_path: PathBuf,
    pub content: String,
}

/// クラッシュリカバリマネージャー
pub struct RecoveryManager {
    backup_dir: PathBuf,
}

impl RecoveryManager {
    /// 新規作成
    pub fn new(root_dir: Option<&Path>) -> Self {
        let base = root_dir.map(|p| p.to_path_buf()).unwrap_or_else(|| PathBuf::from("."));
        let backup_dir = base.join(".nucleus").join("backup");
        let _ = fs::create_dir_all(&backup_dir);
        Self { backup_dir }
    }

    fn path_to_backup_name(path: &str) -> String {
        let clean = path.replace(':', "_").replace('\\', "_").replace('/', "_");
        format!("{}.backup", clean)
    }

    /// 未保存バッファのスナップショット保存
    pub fn save_snapshot(&self, original_path: &str, content: &str) -> std::io::Result<()> {
        let _ = fs::create_dir_all(&self.backup_dir);
        let backup_name = Self::path_to_backup_name(original_path);
        let target = self.backup_dir.join(backup_name);
        
        let payload = serde_json::json!({
            "original_path": original_path,
            "content": content,
            "timestamp": std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs(),
        });

        fs::write(target, serde_json::to_string_pretty(&payload)?)
    }

    /// バッファ保存時またはタブクローズ時にスナップショットを削除
    pub fn remove_snapshot(&self, original_path: &str) {
        let backup_name = Self::path_to_backup_name(original_path);
        let target = self.backup_dir.join(backup_name);
        let _ = fs::remove_file(target);
    }

    /// 起動時に残存している未保存スナップショットをリストアップ
    pub fn list_snapshots(&self) -> Vec<RecoverySnapshot> {
        let mut results = Vec::new();
        if let Ok(entries) = fs::read_dir(&self.backup_dir) {
            for entry in entries.flatten() {
                let p = entry.path();
                if p.is_file() && p.extension().and_then(|e| e.to_str()) == Some("backup") {
                    if let Ok(content_str) = fs::read_to_string(&p) {
                        if let Ok(val) = serde_json::from_str::<serde_json::Value>(&content_str) {
                            if let (Some(orig), Some(text)) = (
                                val.get("original_path").and_then(|o| o.as_str()),
                                val.get("content").and_then(|c| c.as_str()),
                            ) {
                                results.push(RecoverySnapshot {
                                    original_path: orig.to_string(),
                                    backup_file_path: p.clone(),
                                    content: text.to_string(),
                                });
                            }
                        }
                    }
                }
            }
        }
        results
    }

    /// 全スナップショットのクリア
    pub fn clear_all(&self) {
        let _ = fs::remove_dir_all(&self.backup_dir);
        let _ = fs::create_dir_all(&self.backup_dir);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_recovery_snapshot_lifecycle() {
        let temp_dir = std::env::temp_dir().join(format!("nucleus_test_recovery_{}", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()));
        let manager = RecoveryManager::new(Some(&temp_dir));

        let file_path = "src/main.rs";
        let content = "fn main() { println!(\"recovered\"); }";

        manager.save_snapshot(file_path, content).unwrap();

        let snapshots = manager.list_snapshots();
        assert_eq!(snapshots.len(), 1);
        assert_eq!(snapshots[0].original_path, file_path);
        assert_eq!(snapshots[0].content, content);

        manager.remove_snapshot(file_path);
        let snapshots_after = manager.list_snapshots();
        assert_eq!(snapshots_after.len(), 0);

        let _ = fs::remove_dir_all(temp_dir);
    }
}
