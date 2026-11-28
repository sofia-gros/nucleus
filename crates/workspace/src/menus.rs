//! タイトルバーのメニュー（File / Edit / View / Selection / Find / Extensions / Help）と、
//! メニューや Welcome から呼ぶ操作（新しいファイル、クローン、最近のプロジェクト、ヘルプ）。

use std::path::PathBuf;

use gpui_kit::{
    App, AppContext as _, Context, FocusHandle, Focusable as _, IntoElement, ParentElement, Styled, Window,
    component::{
        Sizable as _, WindowExt as _,
        button::{Button, ButtonVariants as _},
        h_flex,
        input::{
            Copy, Cut, GoToDefinition, Indent, Input, InputState, Outdent, Paste, Redo, Replace, Search, SelectAll,
            SelectToEndOfLine, SelectToStartOfLine, Undo,
        },
        menu::{DropdownMenu as _, PopupMenu},
        notification::Notification,
        v_flex,
    },
    div,
};
use i18n::Translate as _;
use serde::Deserialize;
use theme::{Token, TokenColors as _};

use crate::{
    CloseActiveEditor, CommandPalette, FindFile, NewTerminal, OpenFolder, OpenSettings, OpenWorkspaceSettings,
    ProjectSearch, Quit, SaveActiveEditor, SelectLanguage, SelectTheme, ToggleBottomDock, ToggleLeftDock,
    ToggleRightDock, ToggleTerminal, ToggleTheme, Workspace, recent,
};

/// 最近のプロジェクトを開く。
#[derive(Clone, PartialEq, Eq, Deserialize, gpui_kit::Action)]
#[action(namespace = workspace, no_json)]
pub struct OpenRecent {
    pub path: PathBuf,
}

gpui_kit::actions!(
    workspace,
    [
        NewFile,
        CloneRepository,
        About,
        OpenDocumentation,
        ReportIssue,
        ShowKeyboardShortcuts
    ]
);

const REPO_URL: &str = "https://github.com/sofia-gros/nucleus";

/// キーと操作の一覧（Help → Keyboard Shortcuts）。i18n キーはメニューと同じものを使う。
const SHORTCUTS: &[(&str, &str)] = &[
    ("Ctrl+Shift+P / F1", "core.command.command-palette"),
    ("Ctrl+P", "core.command.find-file"),
    ("Ctrl+N", "core.menu-item.new-file"),
    ("Ctrl+O", "core.command.open-folder"),
    ("Ctrl+S", "core.command.save"),
    ("Ctrl+W", "core.command.close-tab"),
    ("Ctrl+F / Ctrl+H", "core.menu-item.find-replace"),
    ("F3 / Shift+F3", "core.menu-item.find-next-previous"),
    ("Ctrl+Shift+F", "core.command.project-search"),
    ("F12", "core.menu-item.go-to-definition"),
    ("Ctrl+`", "core.command.toggle-terminal"),
    ("Ctrl+Shift+T", "core.command.new-terminal"),
    ("Ctrl+B / Ctrl+J / Ctrl+Alt+B", "core.menu-item.toggle-docks"),
    ("Ctrl+Shift+X", "core.command.open-extensions"),
    ("Ctrl+,", "core.command.open-settings"),
    ("Ctrl+Q", "core.command.quit"),
];

impl Workspace {
    /// メニューの操作を送る先（開いているエディタがあればそこ、なければワークスペース）。
    fn menu_context(&self, window: &Window, cx: &App) -> FocusHandle {
        let _ = window;
        self.active_editor(cx)
            .map(|e| e.read(cx).state().focus_handle(cx))
            .unwrap_or_else(|| self.focus.clone())
    }

    pub(crate) fn render_menus(&self, window: &Window, cx: &mut Context<Self>) -> impl IntoElement {
        let ctx = self.menu_context(window, cx);
        let recent: Vec<PathBuf> = cx
            .try_global::<recent::RecentProjects>()
            .map(|r| r.0.clone())
            .unwrap_or_default();
        let t = |key: &str, cx: &App| cx.t(key);
        let menu = |id: &'static str, cx: &mut Context<Self>| {
            Button::new(id).ghost().small().label(cx.t(&format!("core.menu.{id}")))
        };
        let c = ctx.clone();
        let file = menu("file", cx).dropdown_menu(move |m, window, cx| {
            let mut m = m
                .action_context(c.clone())
                .menu(t("core.menu-item.new-file", cx), Box::new(NewFile))
                .menu(t("core.command.open-folder", cx), Box::new(OpenFolder))
                .menu(t("core.welcome.clone", cx), Box::new(CloneRepository));
            if !recent.is_empty() {
                let items = recent.clone();
                m = m.submenu(
                    t("core.menu-item.open-recent", cx),
                    window,
                    cx,
                    move |mut sub: PopupMenu, _, _| {
                        for p in &items {
                            sub = sub.menu(recent::display(p), Box::new(OpenRecent { path: p.clone() }));
                        }
                        sub
                    },
                );
            }
            m.separator()
                .menu(t("core.command.save", cx), Box::new(SaveActiveEditor))
                .menu(t("core.command.close-tab", cx), Box::new(CloseActiveEditor))
                .separator()
                .menu(t("core.command.open-settings", cx), Box::new(OpenSettings))
                .menu(
                    t("core.command.open-workspace-settings", cx),
                    Box::new(OpenWorkspaceSettings),
                )
                .separator()
                .menu(t("core.command.quit", cx), Box::new(Quit))
        });
        let c = ctx.clone();
        let edit = menu("edit", cx).dropdown_menu(move |m, _, cx| {
            m.action_context(c.clone())
                .menu(t("core.menu-item.undo", cx), Box::new(Undo))
                .menu(t("core.menu-item.redo", cx), Box::new(Redo))
                .separator()
                .menu(t("core.menu-item.cut", cx), Box::new(Cut))
                .menu(t("core.menu-item.copy", cx), Box::new(Copy))
                .menu(t("core.menu-item.paste", cx), Box::new(Paste))
                .separator()
                .menu(t("core.menu-item.indent", cx), Box::new(Indent))
                .menu(t("core.menu-item.outdent", cx), Box::new(Outdent))
        });
        let c = ctx.clone();
        let view = menu("view", cx).dropdown_menu(move |m, _, cx| {
            m.action_context(c.clone())
                .menu(t("core.command.command-palette", cx), Box::new(CommandPalette))
                .separator()
                .menu(t("core.command.toggle-left-dock", cx), Box::new(ToggleLeftDock))
                .menu(t("core.command.toggle-bottom-dock", cx), Box::new(ToggleBottomDock))
                .menu(t("core.command.toggle-right-dock", cx), Box::new(ToggleRightDock))
                .separator()
                .menu(t("core.command.toggle-terminal", cx), Box::new(ToggleTerminal))
                .menu(t("core.command.new-terminal", cx), Box::new(NewTerminal))
                .separator()
                .menu(t("core.command.select-theme", cx), Box::new(SelectTheme))
                .menu(t("core.command.toggle-theme", cx), Box::new(ToggleTheme))
                .menu(t("core.command.select-language", cx), Box::new(SelectLanguage))
        });
        let c = ctx.clone();
        let selection = menu("selection", cx).dropdown_menu(move |m, _, cx| {
            m.action_context(c.clone())
                .menu(t("core.menu-item.select-all", cx), Box::new(SelectAll))
                .menu(
                    t("core.menu-item.select-to-line-start", cx),
                    Box::new(SelectToStartOfLine),
                )
                .menu(t("core.menu-item.select-to-line-end", cx), Box::new(SelectToEndOfLine))
        });
        let c = ctx.clone();
        let find = menu("find", cx).dropdown_menu(move |m, _, cx| {
            m.action_context(c.clone())
                .menu(t("core.menu-item.find", cx), Box::new(Search))
                .menu(t("core.menu-item.replace", cx), Box::new(Replace))
                .menu(t("core.menu-item.find-next", cx), Box::new(editor::FindNext))
                .menu(t("core.menu-item.find-previous", cx), Box::new(editor::FindPrevious))
                .separator()
                .menu(t("core.command.project-search", cx), Box::new(ProjectSearch))
                .menu(t("core.command.find-file", cx), Box::new(FindFile))
                .menu(t("core.menu-item.go-to-definition", cx), Box::new(GoToDefinition))
        });
        let extensions =
            menu("extensions", cx).on_click(cx.listener(|this, _, window, cx| this.open_extensions(window, cx)));
        let c = ctx;
        let help = menu("help", cx).dropdown_menu(move |m, _, cx| {
            m.action_context(c.clone())
                .menu(t("core.menu-item.documentation", cx), Box::new(OpenDocumentation))
                .menu(
                    t("core.menu-item.keyboard-shortcuts", cx),
                    Box::new(ShowKeyboardShortcuts),
                )
                .menu(t("core.menu-item.report-issue", cx), Box::new(ReportIssue))
                .separator()
                .menu(t("core.menu-item.about", cx), Box::new(About))
        });
        h_flex()
            .child(file)
            .child(edit)
            .child(view)
            .child(selection)
            .child(find)
            .child(extensions)
            .child(help)
    }

    /// 新しいファイル: 保存先を選んで空のファイルを作り、開く。
    pub(crate) fn new_file(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let dir = self
            .project
            .read(cx)
            .root()
            .or_else(dirs::home_dir)
            .unwrap_or_else(|| PathBuf::from("."));
        let rx = cx.prompt_for_new_path(&dir, Some("untitled.txt"));
        cx.spawn_in(window, async move |this, cx| {
            let Ok(Ok(Some(path))) = rx.await else { return };
            let created = if path.exists() {
                Ok(())
            } else {
                std::fs::write(&path, "")
            };
            this.update_in(cx, |this, window, cx| match created {
                Ok(()) => this.open_path(path, window, cx),
                Err(e) => window.push_notification(Notification::error(format!("{}: {e}", path.display())), cx),
            })
            .ok();
        })
        .detach();
    }

    /// リポジトリをクローンする: URL を入れてもらい、置き場所のフォルダを選んで `git clone` し、開く。
    pub(crate) fn clone_repository(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let url = cx.new(|cx| InputState::new(window, cx).placeholder("https://github.com/owner/repo.git"));
        let this = cx.entity().downgrade();
        let title = cx.t("core.welcome.clone");
        let ok = cx.t("core.clone.choose-folder");
        let hint = cx.t("core.clone.hint");
        url.update(cx, |s, cx| s.focus(window, cx));
        window.open_dialog(cx, move |dialog, _, cx| {
            let url = url.clone();
            let this = this.clone();
            dialog
                .title(title.clone())
                .w(gpui_kit::px(520.))
                .child(
                    v_flex().gap_2().child(Input::new(&url)).child(
                        div()
                            .text_xs()
                            .text_color(cx.token(Token::MutedForeground))
                            .child(hint.clone()),
                    ),
                )
                .footer(
                    h_flex()
                        .justify_end()
                        .child(
                            Button::new("clone-ok")
                                .primary()
                                .label(ok.clone())
                                .on_click(move |_, window, cx| {
                                    let u = url.read(cx).value().trim().to_string();
                                    if u.is_empty() {
                                        return;
                                    }
                                    window.close_dialog(cx);
                                    this.update(cx, |this, cx| this.clone_into_folder(u, cx)).ok();
                                }),
                        ),
                )
        });
    }

    fn clone_into_folder(&mut self, url: String, cx: &mut Context<Self>) {
        let rx = cx.prompt_for_paths(gpui_kit::PathPromptOptions {
            files: false,
            directories: true,
            multiple: false,
            prompt: Some(cx.t("core.clone.choose-folder")),
        });
        let Some(window) = cx.active_window() else { return };
        cx.spawn(async move |this, cx| {
            let Ok(Ok(Some(dirs))) = rx.await else { return };
            let Some(parent) = dirs.into_iter().next() else { return };
            let name = url
                .trim_end_matches('/')
                .rsplit(['/', ':'])
                .next()
                .unwrap_or("repo")
                .trim_end_matches(".git")
                .to_string();
            let dest = parent.join(&name);
            let started = cx.update_window(window, |_, window, cx| {
                let msg = cx.t_args("core.clone.started", &[("url", url.clone().into())]);
                window.push_notification(Notification::info(msg), cx);
            });
            if started.is_err() {
                return;
            }
            let (u, d) = (url.clone(), dest.clone());
            let result = cx
                .background_spawn(async move {
                    std::process::Command::new("git")
                        .args(["clone", "--", &u])
                        .arg(&d)
                        .env("GIT_TERMINAL_PROMPT", "0")
                        .output()
                })
                .await;
            cx.update_window(window, |_, window, cx| match result {
                Ok(out) if out.status.success() => {
                    this.update(cx, |this, cx| this.open_path(dest.clone(), window, cx))
                        .ok();
                }
                Ok(out) => {
                    let err = String::from_utf8_lossy(&out.stderr).trim().to_string();
                    let msg = cx.t_args("core.clone.failed", &[("error", err.into())]);
                    window.push_notification(Notification::error(msg), cx);
                }
                Err(e) => {
                    let msg = cx.t_args("core.clone.failed", &[("error", e.to_string().into())]);
                    window.push_notification(Notification::error(msg), cx);
                }
            })
            .ok();
        })
        .detach();
    }

    pub(crate) fn show_about(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let version = env!("CARGO_PKG_VERSION");
        let name = cx.t("core.app-name");
        let tagline = cx.t("core.welcome.tagline");
        window.open_dialog(cx, move |dialog, _, cx| {
            dialog.title(name.clone()).w(gpui_kit::px(420.)).child(
                v_flex()
                    .gap_2()
                    .text_sm()
                    .child(div().text_color(cx.token(Token::Foreground)).child(tagline.clone()))
                    .child(
                        div()
                            .text_color(cx.token(Token::MutedForeground))
                            .child(format!("Version {version}")),
                    )
                    .child(div().text_color(cx.token(Token::Link)).child(REPO_URL)),
            )
        });
    }

    pub(crate) fn show_shortcuts(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let title = cx.t("core.menu-item.keyboard-shortcuts");
        let rows: Vec<(String, String)> = SHORTCUTS
            .iter()
            .map(|(k, key)| (k.to_string(), cx.t(key).to_string()))
            .collect();
        window.open_dialog(cx, move |dialog, _, cx| {
            dialog
                .title(title.clone())
                .w(gpui_kit::px(560.))
                .child(v_flex().gap_1().children(rows.iter().map(|(k, label)| {
                    h_flex()
                        .gap_4()
                        .text_sm()
                        .child(
                            div()
                                .w(gpui_kit::px(220.))
                                .font_family("JetBrains Mono")
                                .text_xs()
                                .text_color(cx.token(Token::MutedForeground))
                                .child(k.clone()),
                        )
                        .child(div().text_color(cx.token(Token::Foreground)).child(label.clone()))
                })))
        });
    }

    pub(crate) fn open_documentation(&mut self, cx: &mut Context<Self>) {
        cx.open_url(&format!("{REPO_URL}#readme"));
    }

    pub(crate) fn report_issue(&mut self, cx: &mut Context<Self>) {
        cx.open_url(&format!("{REPO_URL}/issues/new"));
    }

    pub(crate) fn open_recent(&mut self, a: &OpenRecent, window: &mut Window, cx: &mut Context<Self>) {
        if a.path.is_dir() {
            self.open_path(a.path.clone(), window, cx);
        } else {
            let msg = cx.t_args("core.recent.missing", &[("path", a.path.display().to_string().into())]);
            window.push_notification(Notification::warning(msg), cx);
        }
    }
}

/// Welcome の「最近のプロジェクト」。
pub fn recent_rows(cx: &App) -> Vec<(PathBuf, String, String)> {
    cx.try_global::<recent::RecentProjects>()
        .map(|r| {
            r.0.iter()
                .take(6)
                .map(|p| {
                    let name = p
                        .file_name()
                        .map(|n| n.to_string_lossy().into_owned())
                        .unwrap_or_else(|| p.display().to_string());
                    (p.clone(), name, recent::display(p))
                })
                .collect()
        })
        .unwrap_or_default()
}
