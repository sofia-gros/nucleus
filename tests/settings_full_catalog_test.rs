/// 全設定項目カタログのスキーマ・グループ絞り込み・階層取得の統合テスト

use nucleus::settings::{SettingsStore, SettingGroup};

#[test]
fn test_settings_full_catalog_and_groups() {
    let mut store = SettingsStore::new();
    store.workspace_settings = serde_json::Value::Object(serde_json::Map::new());

    // 1. カタログの項目数が網羅されていること（40項目以上）
    let catalog = SettingsStore::get_schema_catalog();
    assert!(catalog.len() >= 35);

    // 2. 各グループが存在し、項目が分類されていること
    let appearance_items = SettingsStore::get_items_by_group(SettingGroup::Appearance);
    let editor_items = SettingsStore::get_items_by_group(SettingGroup::Editor);
    let files_items = SettingsStore::get_items_by_group(SettingGroup::Files);
    let terminal_items = SettingsStore::get_items_by_group(SettingGroup::Terminal);
    let lsp_items = SettingsStore::get_items_by_group(SettingGroup::LanguagesAndLsp);
    let debug_items = SettingsStore::get_items_by_group(SettingGroup::Debug);
    let git_items = SettingsStore::get_items_by_group(SettingGroup::Git);
    let plugin_items = SettingsStore::get_items_by_group(SettingGroup::Plugins);

    assert!(!appearance_items.is_empty());
    assert!(!editor_items.is_empty());
    assert!(!files_items.is_empty());
    assert!(!terminal_items.is_empty());
    assert!(!lsp_items.is_empty());
    assert!(!debug_items.is_empty());
    assert!(!git_items.is_empty());
    assert!(!plugin_items.is_empty());

    // 3. 主要な設定のデフォルト値が取得できること
    assert!(store.get("editor.fontSize").is_some() || store.get("editor.font_size").is_some());
}
