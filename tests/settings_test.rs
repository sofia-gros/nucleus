/// グローバル / ワークスペース階層設定の統合テスト

use nucleus::settings::SettingsStore;

#[test]
fn test_settings_hierarchy_override() {
    let mut store = SettingsStore::new();
    store.global_settings = serde_json::from_str("{\"editor\":{\"tab_size\":4,\"font_size\":14}}").unwrap();
    store.workspace_settings = serde_json::from_str("{\"editor\":{\"tab_size\":2}}").unwrap();

    assert_eq!(store.get("editor.tab_size").and_then(|v| v.as_u64()), Some(2));
    assert_eq!(store.get("editor.font_size").and_then(|v| v.as_u64()), Some(14));
    assert_eq!(store.get_user("editor.tab_size").and_then(|v| v.as_u64()), Some(4));
    assert_eq!(store.get_workspace("editor.tab_size").and_then(|v| v.as_u64()), Some(2));
}
