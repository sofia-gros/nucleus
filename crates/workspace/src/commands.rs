//! コマンドパレットに出すコマンドの一覧。名前は i18n キー、実行はアクション。
//! Extension の `contributes.commands` も最後に加わる。

use gpui_kit::{Action, App, SharedString, assets::IconName};
use i18n::Translate as _;

use crate::extension_bridge::{RunExtensionCommand, command_title};

use crate::{
    CloseActiveEditor, CommandPalette, FindFile, NewTerminal, OpenFolder, OpenSettings, OpenWorkspaceSettings, Quit,
    SaveActiveEditor, SearchInActiveEditor, SelectLanguage, SelectTheme, ToggleBottomDock, ToggleLeftDock,
    ToggleRightDock, ToggleTerminal, ToggleTheme,
};

pub struct CommandDef {
    pub title: SharedString,
    pub icon: IconName,
    pub action: Box<dyn Fn() -> Box<dyn Action>>,
}

pub fn all(cx: &App) -> Vec<CommandDef> {
    let c = |title_key: &'static str, icon: IconName, action: fn() -> Box<dyn Action>| CommandDef {
        title: cx.t(title_key),
        icon,
        action: Box::new(action),
    };
    let mut list = vec![
        c("core.command.find-file", IconName::FileCode, || Box::new(FindFile)),
        c("core.command.open-folder", IconName::FolderOpen, || {
            Box::new(OpenFolder)
        }),
        c("core.command.save", IconName::Save, || Box::new(SaveActiveEditor)),
        c("core.command.find-in-file", IconName::Search, || {
            Box::new(SearchInActiveEditor)
        }),
        c("core.command.close-tab", IconName::X, || Box::new(CloseActiveEditor)),
        c("core.command.toggle-terminal", IconName::Terminal, || {
            Box::new(ToggleTerminal)
        }),
        c("core.command.new-terminal", IconName::Terminal, || {
            Box::new(NewTerminal)
        }),
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
        c("core.command.select-theme", IconName::Palette, || Box::new(SelectTheme)),
        c("core.command.select-language", IconName::Languages, || {
            Box::new(SelectLanguage)
        }),
        c("core.command.command-palette", IconName::Command, || {
            Box::new(CommandPalette)
        }),
        c("core.command.quit", IconName::LogOut, || Box::new(Quit)),
    ];
    if let Some(store) = extension_host::ExtensionStore::get(cx) {
        for ec in store.commands() {
            let action = RunExtensionCommand {
                extension: ec.extension.clone(),
                command: ec.command.id.clone(),
            };
            list.push(CommandDef {
                title: command_title(&ec.extension, &ec.command.title, cx).into(),
                icon: IconName::Puzzle,
                action: Box::new(move || Box::new(action.clone())),
            });
        }
    }
    list
}
