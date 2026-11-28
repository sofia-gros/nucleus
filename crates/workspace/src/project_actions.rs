//! プロジェクトツリーの右クリックメニュー（新しいファイル・フォルダ、名前の変更、削除、パスのコピー、ターミナル）。

use std::path::{Path, PathBuf};

use gpui_kit::{
    AppContext as _, ClipboardItem, Context, Entity, ParentElement, PromptLevel, Styled, Window,
    component::{
        Sizable as _, WindowExt as _,
        button::{Button, ButtonVariants as _},
        h_flex,
        input::{Input, InputEvent, InputState},
        menu::PopupMenu,
        notification::Notification,
        v_flex,
    },
    div,
};
use i18n::Translate as _;
use serde::Deserialize;
use theme::{Token, TokenColors as _};

use crate::project_panel::{ProjectPanel, ProjectPanelEvent};

/// ツリーの項目に対する操作（右クリックメニューの項目）。
#[derive(Clone, PartialEq, Eq, Deserialize, gpui_kit::Action)]
#[action(namespace = project_panel, no_json)]
pub struct TreeAction {
    pub op: TreeOp,
    pub path: PathBuf,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
pub enum TreeOp {
    NewFile,
    NewFolder,
    Rename,
    Delete,
    CopyPath,
    CopyRelativePath,
    OpenInTerminal,
}

/// 右クリックメニュー。`is_dir` ならその中に、ファイルなら同じフォルダに新しく作る。
pub fn menu(m: PopupMenu, path: &Path, is_dir: bool, cx: &gpui_kit::App) -> PopupMenu {
    let a = |op| Box::new(TreeAction { op, path: path.to_path_buf() });
    let t = |k: &str| cx.t(k);
    let mut m = m
        .menu(t("core.tree.new-file"), a(TreeOp::NewFile))
        .menu(t("core.tree.new-folder"), a(TreeOp::NewFolder))
        .separator()
        .menu(t("core.tree.rename"), a(TreeOp::Rename))
        .menu(t("core.tree.delete"), a(TreeOp::Delete))
        .separator()
        .menu(t("core.tree.copy-path"), a(TreeOp::CopyPath))
        .menu(t("core.tree.copy-relative-path"), a(TreeOp::CopyRelativePath));
    if is_dir {
        m = m.separator().menu(t("core.tree.open-in-terminal"), a(TreeOp::OpenInTerminal));
    }
    m
}

/// 名前に使えるか（空・区切り文字・`.` / `..` は不可）。
fn valid_name(name: &str) -> bool {
    !name.is_empty() && name != "." && name != ".." && !name.contains(['/', '\\', '\0'])
}

impl ProjectPanel {
    pub(crate) fn tree_action(&mut self, a: &TreeAction, window: &mut Window, cx: &mut Context<Self>) {
        let path = a.path.clone();
        let dir = if path.is_dir() {
            path.clone()
        } else {
            path.parent().map(Path::to_path_buf).unwrap_or_default()
        };
        match a.op {
            TreeOp::NewFile | TreeOp::NewFolder => {
                let folder = a.op == TreeOp::NewFolder;
                let title = cx.t(if folder { "core.tree.new-folder" } else { "core.tree.new-file" });
                self.ask_name(title, String::new(), window, cx, move |this, name, window, cx| {
                    let target = dir.join(name);
                    if target.exists() {
                        return Err(cx.t_args("core.tree.exists", &[("name", name.to_string().into())]).into());
                    }
                    if folder {
                        std::fs::create_dir_all(&target).map_err(|e| e.to_string())?;
                    } else {
                        std::fs::write(&target, "").map_err(|e| e.to_string())?;
                        cx.emit(ProjectPanelEvent::OpenFile(target.clone()));
                    }
                    this.refresh_dir(&dir, cx);
                    let _ = window;
                    Ok(())
                });
            }
            TreeOp::Rename => {
                let old_name = path
                    .file_name()
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or_default();
                let parent = path.parent().map(Path::to_path_buf).unwrap_or_default();
                let title = cx.t("core.tree.rename");
                self.ask_name(title, old_name.clone(), window, cx, move |this, name, _, cx| {
                    if name == old_name {
                        return Ok(());
                    }
                    let target = parent.join(name);
                    if target.exists() {
                        return Err(cx.t_args("core.tree.exists", &[("name", name.to_string().into())]).into());
                    }
                    std::fs::rename(&path, &target).map_err(|e| e.to_string())?;
                    cx.emit(ProjectPanelEvent::Renamed(path.clone(), target));
                    this.refresh_dir(&parent, cx);
                    Ok(())
                });
            }
            TreeOp::Delete => {
                let name = path
                    .file_name()
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or_default();
                let msg = cx.t_args("core.tree.confirm-delete", &[("name", name.into())]);
                let detail = cx.t("core.tree.delete-detail");
                let answers = [cx.t("core.tree.delete"), cx.t("core.prompt.cancel")];
                let answer = window.prompt(
                    PromptLevel::Warning,
                    &msg,
                    Some(&detail),
                    &answers.iter().map(|a| a.as_ref()).collect::<Vec<&str>>(),
                    cx,
                );
                cx.spawn_in(window, async move |this, cx| {
                    if answer.await != Ok(0) {
                        return;
                    }
                    let p = path.clone();
                    let r = cx.background_spawn(async move { trash::delete(&p) }).await;
                    this.update_in(cx, |this, window, cx| match r {
                        Ok(()) => {
                            cx.emit(ProjectPanelEvent::Deleted(path.clone()));
                            if let Some(parent) = path.parent() {
                                this.refresh_dir(parent, cx);
                            }
                        }
                        Err(e) => window.push_notification(Notification::error(e.to_string()), cx),
                    })
                    .ok();
                })
                .detach();
            }
            TreeOp::CopyPath => cx.write_to_clipboard(ClipboardItem::new_string(path.display().to_string())),
            TreeOp::CopyRelativePath => {
                let rel = self
                    .root()
                    .and_then(|r| path.strip_prefix(&r).ok().map(Path::to_path_buf))
                    .unwrap_or(path);
                cx.write_to_clipboard(ClipboardItem::new_string(rel.display().to_string()))
            }
            TreeOp::OpenInTerminal => cx.emit(ProjectPanelEvent::OpenTerminal(dir)),
        }
    }

    /// 名前を入れてもらう。`apply` が Err を返したらダイアログを閉じずにエラーを出す。
    fn ask_name(
        &mut self,
        title: gpui_kit::SharedString,
        initial: String,
        window: &mut Window,
        cx: &mut Context<Self>,
        apply: impl Fn(&mut Self, &str, &mut Window, &mut Context<Self>) -> Result<(), String> + 'static,
    ) {
        let input: Entity<InputState> = cx.new(|cx| InputState::new(window, cx).default_value(initial));
        input.update(cx, |s, cx| {
            s.focus(window, cx);
            s.select_all(window, cx);
        });
        let error: Entity<Option<String>> = cx.new(|_| None);
        let this = cx.entity().downgrade();
        let apply = std::rc::Rc::new(apply);
        let submit = {
            let (input, error, this, apply) = (input.clone(), error.clone(), this.clone(), apply.clone());
            move |window: &mut Window, cx: &mut gpui_kit::App| {
                let name = input.read(cx).value().trim().to_string();
                let result = if valid_name(&name) {
                    this.update(cx, |this, cx| apply(this, &name, window, cx))
                        .unwrap_or(Ok(()))
                } else {
                    Err(i18n::Translate::t(cx, "core.tree.invalid-name").to_string())
                };
                match result {
                    Ok(()) => window.close_dialog(cx),
                    Err(e) => error.update(cx, |v, cx| {
                        *v = Some(e);
                        cx.notify();
                    }),
                }
            }
        };
        let submit = std::rc::Rc::new(submit);
        {
            let submit = submit.clone();
            cx.subscribe_in(&input, window, move |_, _, e: &InputEvent, window, cx| {
                if matches!(e, InputEvent::PressEnter { .. }) {
                    submit(window, cx);
                }
            })
            .detach();
        }
        let ok = cx.t("core.tree.ok");
        window.open_dialog(cx, move |dialog, _, cx| {
            let submit = submit.clone();
            let message = error.read(cx).clone();
            dialog
                .title(title.clone())
                .w(gpui_kit::px(420.))
                .child(
                    v_flex().gap_2().child(Input::new(&input)).children(message.map(|m| {
                        div()
                            .text_xs()
                            .text_color(cx.token(Token::HighlightError))
                            .child(m)
                    })),
                )
                .footer(
                    h_flex().justify_end().child(
                        Button::new("name-ok")
                            .primary()
                            .small()
                            .label(ok.clone())
                            .on_click(move |_, window, cx| submit(window, cx)),
                    ),
                )
        });
    }
}

#[cfg(test)]
mod tests {
    use super::valid_name;

    #[test]
    fn names_cannot_escape_the_folder() {
        assert!(valid_name("a.rs"));
        assert!(!valid_name(""));
        assert!(!valid_name(".."));
        assert!(!valid_name("a/b"));
    }
}
