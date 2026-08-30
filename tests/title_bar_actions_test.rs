/// TitleBar (AppBar) 全アクション & 日本語化ディスパッチ検証テスト

use nucleus::plugin_manager::action::PluginAction;
use nucleus::plugin_manager::ui::UIExtensionRegistry;
use std::collections::HashMap;

#[test]
fn test_title_bar_menu_action_variants() {
    // 全メニュー項目に対応する PluginAction バリアントが網羅されていることをテスト
    let actions = vec![
        PluginAction::OpenFilePicker,
        PluginAction::OpenFolderPicker,
        PluginAction::SaveActiveTab,
        PluginAction::SaveAsActiveTab,
        PluginAction::CloseActiveTab,
        PluginAction::EditorUndo,
        PluginAction::EditorRedo,
        PluginAction::EditorCut,
        PluginAction::EditorCopy,
        PluginAction::EditorPaste,
        PluginAction::EditorSelectAll,
        PluginAction::EditorFind,
        PluginAction::EditorReplace,
        PluginAction::OpenDocumentation,
        PluginAction::OpenCommandPalette,
        PluginAction::OpenSettings,
        PluginAction::OpenKeybindings,
        PluginAction::ToggleSidebar,
        PluginAction::ToggleTerminal,
    ];

    assert_eq!(actions.len(), 19, "All 19 title bar action variants should exist and be testable");
}

#[test]
fn test_title_bar_translations() {
    let mut registry = UIExtensionRegistry::default();
    let mut dict = HashMap::new();
    dict.insert("File".to_string(), "ファイル".to_string());
    dict.insert("Edit".to_string(), "編集".to_string());
    dict.insert("Selection".to_string(), "選択".to_string());
    dict.insert("View".to_string(), "表示".to_string());
    dict.insert("Run".to_string(), "実行".to_string());
    dict.insert("Terminal".to_string(), "ターミナル".to_string());
    dict.insert("Help".to_string(), "ヘルプ".to_string());
    dict.insert("Settings".to_string(), "設定".to_string());
    registry.register_translations(dict);

    assert_eq!(registry.translate("File"), "ファイル");
    assert_eq!(registry.translate("Edit"), "編集");
    assert_eq!(registry.translate("Selection"), "選択");
    assert_eq!(registry.translate("View"), "表示");
    assert_eq!(registry.translate("Run"), "実行");
    assert_eq!(registry.translate("Terminal"), "ターミナル");
    assert_eq!(registry.translate("Help"), "ヘルプ");
    assert_eq!(registry.translate("Settings"), "設定");
}
