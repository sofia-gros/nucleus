/// コマンドパレット (Ctrl+Shift+P) & クイックファイルオープン (Ctrl+P) の統合テスト

use nucleus::workspace::command_palette::fuzzy::fuzzy_match;
use nucleus::workspace::command_palette::PaletteItem;

#[test]
fn test_command_palette_modes_and_fuzzy_search() {
    let commands = vec![
        PaletteItem::Command { category: "Git".into(), title: "Commit".into(), command: "git.commit".into(), shortcut: None },
        PaletteItem::Command { category: "Git".into(), title: "Refresh Status".into(), command: "git.refresh".into(), shortcut: None },
        PaletteItem::Command { category: "File".into(), title: "Save".into(), command: "file.save".into(), shortcut: Some("Ctrl+S".into()) },
    ];

    // 1. コマンドパレット (Ctrl+Shift+P) ファジー検索
    let query = "git";
    let mut matched_commands = Vec::new();
    for cmd in &commands {
        if let PaletteItem::Command { category, title, .. } = cmd {
            let target = format!("{}: {}", category, title);
            if let Some(m) = fuzzy_match(query, &target) {
                matched_commands.push((m.score, cmd));
            }
        }
    }
    matched_commands.sort_by(|a, b| b.0.cmp(&a.0));
    assert_eq!(matched_commands.len(), 2);

    // 2. クイックファイルオープン (Ctrl+P) ファイル検索
    let files = vec![
        PaletteItem::File { path: "src/main.rs".into(), file_name: "main.rs".into(), dir: "src".into() },
        PaletteItem::File { path: "src/lib.rs".into(), file_name: "lib.rs".into(), dir: "src".into() },
        PaletteItem::File { path: "Cargo.toml".into(), file_name: "Cargo.toml".into(), dir: "".into() },
    ];

    let file_query = "main";
    let mut matched_files = Vec::new();
    for f in &files {
        if let PaletteItem::File { file_name, dir, .. } = f {
            let target = format!("{}/{}", dir, file_name);
            if let Some(m) = fuzzy_match(file_query, &target) {
                matched_files.push((m.score, f));
            }
        }
    }
    matched_files.sort_by(|a, b| b.0.cmp(&a.0));
    assert_eq!(matched_files.len(), 1);
}
