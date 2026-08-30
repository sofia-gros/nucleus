/// 設定ストアの即時更新・階層マージ・ターゲット保存の検証テスト

use nucleus::settings::{SettingsStore, SettingsTarget};

#[test]
fn test_settings_live_target_persistence() {
    let mut store = SettingsStore::new();
    store.workspace_settings = serde_json::Value::Object(serde_json::Map::new());

    // 1. グローバル (User) 設定の更新
    store.set_target(SettingsTarget::User, "test.theme", serde_json::json!("light"));
    store.set_target(SettingsTarget::User, "test.editor.font_size", serde_json::json!(16));

    let theme = store.get("test.theme").and_then(|v| v.as_str().map(|s| s.to_string()));
    let font_size = store.get("test.editor.font_size").and_then(|v| v.as_u64());

    assert_eq!(theme.as_deref(), Some("light"));
    assert_eq!(font_size, Some(16));

    // 2. ワークスペース設定でのオーバーライド
    store.set_target(SettingsTarget::Workspace, "test.editor.font_size", serde_json::json!(20));
    let font_size_overridden = store.get("test.editor.font_size").and_then(|v| v.as_u64());

    assert_eq!(font_size_overridden, Some(20));

    // 3. ワークスペースに存在しない項目はグローバルが維持される
    let theme_still_user = store.get("test.theme").and_then(|v| v.as_str().map(|s| s.to_string()));
    assert_eq!(theme_still_user.as_deref(), Some("light"));
}
