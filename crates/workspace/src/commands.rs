//! コマンドパレットに出すコマンドの一覧。名前は i18n キー、実行はアクション。
//! 将来は Extension の `contributes.commands` もここに加わる。

use gpui_kit::{Action, assets::IconName};

use crate::{
    CloseActiveEditor, CommandPalette, FindFile, OpenFolder, OpenSettings, OpenWorkspaceSettings, Quit,
    SaveActiveEditor, SearchInActiveEditor, ToggleBottomDock, ToggleLeftDock, ToggleRightDock, ToggleTheme, UseEnglish,
    UseJapanese,
};

pub struct CommandDef {
    pub title_key: &'static str,
    pub icon: IconName,
    pub action: fn() -> Box<dyn Action>,
}

pub fn all() -> Vec<CommandDef> {
    fn c(title_key: &'static str, icon: IconName, action: fn() -> Box<dyn Action>) -> CommandDef {
        CommandDef {
            title_key,
            icon,
            action,
        }
    }
    vec![
        c("core.command.find-file", IconName::FileCode, || Box::new(FindFile)),
        c("core.command.open-folder", IconName::FolderOpen, || {
            Box::new(OpenFolder)
        }),
        c("core.command.save", IconName::Save, || Box::new(SaveActiveEditor)),
        c("core.command.find-in-file", IconName::Search, || {
            Box::new(SearchInActiveEditor)
        }),
        c("core.command.close-tab", IconName::X, || Box::new(CloseActiveEditor)),
        c("core.command.toggle-left-dock", IconName::PanelLeft, || {
            Box::new(ToggleLeftDock)
        }),
        c("core.command.toggle-bottom-dock", IconName::PanelBottom, || {
            Box::new(ToggleBottomDock)
        }),
        c("core.command.toggle-right-dock", IconName::PanelRight, || {
            Box::new(ToggleRightDock)
        }),
        c("core.command.open-settings", IconName::Settings2, || {
            Box::new(OpenSettings)
        }),
        c("core.command.open-workspace-settings", IconName::Settings2, || {
            Box::new(OpenWorkspaceSettings)
        }),
        c("core.command.toggle-theme", IconName::Palette, || Box::new(ToggleTheme)),
        c("core.command.language-en", IconName::Languages, || Box::new(UseEnglish)),
        c("core.command.language-ja", IconName::Languages, || {
            Box::new(UseJapanese)
        }),
        c("core.command.command-palette", IconName::Command, || {
            Box::new(CommandPalette)
        }),
        c("core.command.quit", IconName::LogOut, || Box::new(Quit)),
    ]
}
