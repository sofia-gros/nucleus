/// プラグインによる UI 拡張レジストリおよび UI AST 定義モジュール

use serde_json::Value;

/// Activity Bar（左端の縦アイコンバー）の拡張アイテム
#[derive(Clone, Debug)]
pub struct ActivityBarItem {
    pub id: String,
    pub plugin_id: String,
    pub icon: String,
    pub tooltip: String,
    pub command: String,
}

/// Status Bar（下部ステータスバー）の拡張アイテム
#[derive(Clone, Debug)]
pub struct StatusBarItem {
    pub id: String,
    pub plugin_id: String,
    pub text: String,
    pub icon: Option<String>,
    pub command: Option<String>,
    pub alignment: StatusBarAlignment,
}

/// Status Bar の配置位置
#[derive(Clone, Debug, PartialEq)]
pub enum StatusBarAlignment {
    Left,
    Right,
}

/// サイドバーのカスタムビュー拡張アイテム
#[derive(Clone, Debug)]
pub struct SidebarItem {
    pub id: String,
    pub plugin_id: String,
    pub title: String,
    pub ui_ast: Value,
}

/// ボトムパネルのカスタムビュー拡張アイテム
#[derive(Clone, Debug)]
pub struct PanelItem {
    pub id: String,
    pub plugin_id: String,
    pub title: String,
    pub ui_ast: Value,
}

use std::collections::HashMap;

/// ファイルツリーの合成装飾データ (Material Icons + Git 等の複数プラグイン合成)
#[derive(Clone, Debug, Default)]
pub struct FileItemDecoration {
    pub icon_text: Option<String>,
    pub icon_color: Option<String>,
    pub status_badge: Option<String>,
    pub status_color: Option<String>,
}

/// プラグインからの UI 拡張登録を保持するレジストリ
#[derive(Default)]
pub struct UIExtensionRegistry {
    pub activity_bar_items: Vec<ActivityBarItem>,
    pub status_bar_items: Vec<StatusBarItem>,
    pub sidebar_items: Vec<SidebarItem>,
    pub panel_items: Vec<PanelItem>,
    pub file_icon_rules: HashMap<String, (String, String)>, // 拡張子/パターン -> (アイコン文字, カラーHEX)
    pub translations: HashMap<String, String>, // 言語パック翻訳辞書
}

impl UIExtensionRegistry {
    /// Activity Bar アイテムを登録
    pub fn register_activity_bar_item(&mut self, item: ActivityBarItem) {
        self.activity_bar_items.retain(|i| i.id != item.id);
        self.activity_bar_items.push(item);
    }

    /// Status Bar アイテムを登録（既存IDは上書き更新）
    pub fn register_status_bar_item(&mut self, item: StatusBarItem) {
        self.status_bar_items.retain(|i| i.id != item.id);
        self.status_bar_items.push(item);
    }

    /// サイドバーアイテムを登録
    pub fn register_sidebar_item(&mut self, item: SidebarItem) {
        self.sidebar_items.retain(|i| i.id != item.id);
        self.sidebar_items.push(item);
    }

    /// 既存のサイドバーアイテムを動的更新
    pub fn update_sidebar_item(&mut self, plugin_id: &str, id: &str, title: Option<String>, ui_ast: Value) {
        if let Some(item) = self.sidebar_items.iter_mut().find(|i| i.id == id) {
            if let Some(t) = title {
                item.title = t;
            }
            item.ui_ast = ui_ast;
        } else {
            self.sidebar_items.push(SidebarItem {
                id: id.to_string(),
                plugin_id: plugin_id.to_string(),
                title: title.unwrap_or_else(|| "Sidebar".to_string()),
                ui_ast,
            });
        }
    }

    /// パネルアイテムを登録
    pub fn register_panel_item(&mut self, item: PanelItem) {
        self.panel_items.retain(|i| i.id != item.id);
        self.panel_items.push(item);
    }

    /// アイコンルールを登録 (material_icons プラグイン等から)
    pub fn register_icon_rules(&mut self, rules: HashMap<String, (String, String)>) {
        for (ext, val) in rules {
            self.file_icon_rules.insert(ext, val);
        }
    }

    /// 翻訳辞書をマージ登録 (japanese_language プラグイン等から)
    pub fn register_translations(&mut self, dict: HashMap<String, String>) {
        for (k, v) in dict {
            self.translations.insert(k, v);
        }
    }

    /// キーに対応する翻訳文言の取得（未登録時はキーそのまま）
    pub fn translate<'a>(&'a self, key: &'a str) -> &'a str {
        self.translations.get(key).map(|s| s.as_str()).unwrap_or(key)
    }

    /// ファイルパスに対する複数プラグインの装飾を競合なくマージ合成
    pub fn merge_file_decorations(
        &self,
        path_str: &str,
        is_dir: bool,
        git_status_map: &HashMap<String, String>,
    ) -> FileItemDecoration {
        let mut deco = FileItemDecoration::default();

        // 1. Material Icons プラグインによるアイコン解決
        if is_dir {
            deco.icon_text = Some("📁".to_string());
            deco.icon_color = Some("#90caf9".to_string());
        } else {
            let norm = path_str.replace('\\', "/");
            let file_name = norm.split('/').last().unwrap_or("");
            
            // 完全ファイル名照合 (例: Cargo.toml, Dockerfile, package.json)
            if let Some((icon, color)) = self.file_icon_rules.get(file_name) {
                deco.icon_text = Some(icon.clone());
                deco.icon_color = Some(color.clone());
            } else if let Some(ext) = file_name.split('.').last() {
                // 拡張子照合 (例: rs, ts, js, md)
                if let Some((icon, color)) = self.file_icon_rules.get(ext) {
                    deco.icon_text = Some(icon.clone());
                    deco.icon_color = Some(color.clone());
                } else {
                    deco.icon_text = Some("📄".to_string());
                    deco.icon_color = Some("#cccccc".to_string());
                }
            } else {
                deco.icon_text = Some("📄".to_string());
                deco.icon_color = Some("#cccccc".to_string());
            }
        }

        // 2. Git プラグインによるステータスバッジ解決 (上書きせず共存)
        let norm_path_str = path_str.replace('\\', "/");
        if !is_dir {
            if let Some(status) = git_status_map.get(&norm_path_str) {
                deco.status_badge = Some(status.clone());
                deco.status_color = Some(match status.as_str() {
                    "M" | "MM" => "#eab308".to_string(),
                    "U" | "??" | "A" => "#22c55e".to_string(),
                    "D" => "#ef4444".to_string(),
                    _ => "#888888".to_string(),
                });
            } else {
                for (node_path, status) in git_status_map {
                    if norm_path_str.ends_with(&format!("/{}", node_path.trim_start_matches('/')))
                        || node_path.ends_with(&format!("/{}", norm_path_str.trim_start_matches('/')))
                    {
                        deco.status_badge = Some(status.clone());
                        deco.status_color = Some(match status.as_str() {
                            "M" | "MM" => "#eab308".to_string(),
                            "U" | "??" | "A" => "#22c55e".to_string(),
                            "D" => "#ef4444".to_string(),
                            _ => "#888888".to_string(),
                        });
                        break;
                    }
                }
            }
        } else {
            let trimmed_file = norm_path_str.trim_end_matches('/');
            for (node_path, _) in git_status_map {
                let trimmed_git = node_path.trim_start_matches('/');
                if trimmed_git.starts_with(trimmed_file) {
                    deco.status_badge = Some("●".to_string());
                    deco.status_color = Some("#eab308".to_string());
                    break;
                }
            }
        }

        deco
    }
}
