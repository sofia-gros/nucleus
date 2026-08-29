/// クラッシュリカバリと自動スナップショットの統合テスト

use nucleus::workspace::recovery::RecoveryManager;
use std::fs;

#[test]
fn test_recovery_snapshot_lifecycle() {
    let temp_dir = std::env::temp_dir().join(format!("nucleus_test_recovery_{}", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()));
    let manager = RecoveryManager::new(Some(&temp_dir));

    let file_path = "src/main.rs";
    let content = "fn main() { println!(\"recovered content\"); }";

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
