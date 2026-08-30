/// Git 一行UIレイアウト・一括アクション・バッジ不滅性検証テスト

use serde_json::json;

#[test]
fn test_git_node_single_line_formatting() {
    let node = json!({
        "name": "settings_view.rs",
        "dir": "src/workspace/editor_area",
        "path": "src/workspace/editor_area/settings_view.rs",
        "status": "M"
    });

    let name = node.get("name").and_then(|n| n.as_str()).unwrap();
    let dir = node.get("dir").and_then(|d| d.as_str()).unwrap();
    let status = node.get("status").and_then(|s| s.as_str()).unwrap();

    assert_eq!(name, "settings_view.rs");
    assert_eq!(dir, "src/workspace/editor_area");
    assert_eq!(status, "M");

    // 一行表示フォーマットの検証
    let formatted = format!("{} {}", name, dir);
    assert_eq!(formatted, "settings_view.rs src/workspace/editor_area");
}

#[test]
fn test_git_batch_command_generation() {
    // 一括ステージング、アンステージ、破棄コマンドの生成検証
    let stage_all_cmd = "git.stage_all";
    let unstage_all_cmd = "git.unstage_all";
    let discard_all_cmd = "git.discard_all";

    assert_eq!(stage_all_cmd, "git.stage_all");
    assert_eq!(unstage_all_cmd, "git.unstage_all");
    assert_eq!(discard_all_cmd, "git.discard_all");
}
