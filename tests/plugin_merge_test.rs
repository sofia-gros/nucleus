/// プラグインUIマージ合成パイプライン & 言語パック検証テスト

use std::collections::HashMap;
use nucleus::plugin_manager::ui::UIExtensionRegistry;

#[test]
fn test_plugin_ui_merge_decorations() {
    let mut registry = UIExtensionRegistry::default();

    // 1. Material Icons プラグインからのルール登録
    let mut icon_rules = HashMap::new();
    icon_rules.insert("rs".to_string(), ("🦀".to_string(), "#f97316".to_string()));
    icon_rules.insert("toml".to_string(), ("⚙️".to_string(), "#94a3b8".to_string()));
    icon_rules.insert("Cargo.toml".to_string(), ("📦".to_string(), "#ea580c".to_string()));
    registry.register_icon_rules(icon_rules);

    // 2. Git プラグインからのステータス照合マップ
    let mut git_map = HashMap::new();
    git_map.insert("src/main.rs".to_string(), "M".to_string());
    git_map.insert("Cargo.toml".to_string(), "U".to_string());

    // 3. マージ合成の検証: src/main.rs (Rust アイコン 🦀 + Git 変更バッジ M)
    let deco_rs = registry.merge_file_decorations("src/main.rs", false, &git_map);
    assert_eq!(deco_rs.icon_text, Some("🦀".to_string()), "Rust icon should be provided by Material Icons");
    assert_eq!(deco_rs.icon_color, Some("#f97316".to_string()));
    assert_eq!(deco_rs.status_badge, Some("M".to_string()), "Git status badge should be provided by Git plugin without conflict");
    assert_eq!(deco_rs.status_color, Some("#eab308".to_string()));

    // 4. マージ合成の検証: Cargo.toml (特別アイコン 📦 + Git 新規バッジ U)
    let deco_cargo = registry.merge_file_decorations("Cargo.toml", false, &git_map);
    assert_eq!(deco_cargo.icon_text, Some("📦".to_string()));
    assert_eq!(deco_cargo.status_badge, Some("U".to_string()));
    assert_eq!(deco_cargo.status_color, Some("#22c55e".to_string()));

    // 5. ディレクトリのマージ合成
    let deco_dir = registry.merge_file_decorations("src", true, &git_map);
    assert_eq!(deco_dir.icon_text, Some("📁".to_string()));
    assert_eq!(deco_dir.status_badge, Some("●".to_string()), "Directory should inherit git modified dot");
}

#[test]
fn test_japanese_language_pack_translations() {
    let mut registry = UIExtensionRegistry::default();

    let mut dict = HashMap::new();
    dict.insert("File".to_string(), "ファイル".to_string());
    dict.insert("Settings".to_string(), "設定".to_string());
    dict.insert("Appearance".to_string(), "外観".to_string());
    registry.register_translations(dict);

    assert_eq!(registry.translate("File"), "ファイル");
    assert_eq!(registry.translate("Settings"), "設定");
    assert_eq!(registry.translate("Appearance"), "外観");
    assert_eq!(registry.translate("UnknownKey"), "UnknownKey", "Fallback to original key if not translated");
}
