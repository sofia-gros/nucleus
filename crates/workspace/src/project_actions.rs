//! プロジェクトツリーの右クリックメニュー（新しいファイル・フォルダ、名前の変更、削除、パスのコピー、ターミナル）。

use std::path::{Path, PathBuf};

use gpui_kit::{
    App, AppContext as _, ClipboardItem, Context, Entity, PromptLevel, Window,
    component::{
        WindowExt as _,
        input::{InputEvent, InputState},
        menu::PopupMenu,
        notification::Notification,
    },
};
use i18n::Translate as _;
use project::EntryKind;
use remote::RemoteExt as _;
use serde::Deserialize;

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
/// 開いているフォルダそのもの（`is_root`）には、名前の変更とごみ箱を出さない。
pub fn menu(m: PopupMenu, path: &Path, is_dir: bool, is_root: bool, cx: &gpui_kit::App) -> PopupMenu {
    let a = |op| {
        Box::new(TreeAction {
            op,
            path: path.to_path_buf(),
        })
    };
    let t = |k: &str| cx.t(k);
    let mut m = m
        .menu(t("core.tree.new-file"), a(TreeOp::NewFile))
        .menu(t("core.tree.new-folder"), a(TreeOp::NewFolder))
        .separator();
    if !is_root {
        m = m
            .menu(t("core.tree.rename"), a(TreeOp::Rename))
            .menu(t("core.tree.delete"), a(TreeOp::Delete))
            .separator();
    }
    let mut m = m
        .menu(t("core.tree.copy-path"), a(TreeOp::CopyPath))
        .menu(t("core.tree.copy-relative-path"), a(TreeOp::CopyRelativePath));
    if is_dir {
        m = m
            .separator()
            .menu(t("core.tree.open-in-terminal"), a(TreeOp::OpenInTerminal));
    }
    m
}

/// 名前に使えるか（空・区切り文字・`.` / `..` は不可）。
fn valid_name(name: &str) -> bool {
    !name.is_empty() && name != "." && name != ".." && !name.contains(['/', '\\', '\0'])
}

impl ProjectPanel {
    pub(crate) fn tree_action(&mut self, a: &TreeAction, window: &mut Window, cx: &mut Context<Self>) {
        self.cancel_edit(cx);
        let path = a.path.clone();
        // 開いているフォルダそのものは消したり名前を変えたりしない（メニューにも出さない）。
        if matches!(a.op, TreeOp::Rename | TreeOp::Delete) && self.root().as_ref() == Some(&path) {
            return;
        }
        let dir = if path.is_dir() {
            path.clone()
        } else {
            path.parent().map(Path::to_path_buf).unwrap_or_default()
        };
        match a.op {
            TreeOp::NewFile => self.start_edit(EditKind::NewFile, dir, None, window, cx),
            TreeOp::NewFolder => self.start_edit(EditKind::NewFolder, dir, None, window, cx),
            TreeOp::Rename => {
                let parent = path.parent().map(Path::to_path_buf).unwrap_or_default();
                self.start_edit(EditKind::Rename, parent, Some(path), window, cx)
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
                    let r = this.update(cx, |_, cx| cx.remote().trash(&path)).ok();
                    let Some(r) = r else { return };
                    let r = r.await;
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

    /// ツリーの中で名前を入れ始める（VSCode と同じ）。新しく作るときはフォルダを開いて先頭に入力欄を出し、
    /// 名前を変えるときはその行を入力欄にする。Enter かフォーカスが外れたら確定、Esc で取り消す。
    fn start_edit(
        &mut self,
        kind: EditKind,
        dir: PathBuf,
        target: Option<PathBuf>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if kind != EditKind::Rename {
            self.expand(&dir, cx);
        }
        let initial = target
            .as_ref()
            .and_then(|t| t.file_name())
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        let input: Entity<InputState> = cx.new(|cx| InputState::new(window, cx).default_value(initial.clone()));
        let sub = cx.subscribe_in(&input, window, |this, _, e: &InputEvent, _, cx| match e {
            InputEvent::PressEnter { .. } => this.commit_edit(false, cx),
            InputEvent::Blur => this.commit_edit(true, cx),
            InputEvent::Change => {
                if let Some(edit) = this.editing.as_mut() {
                    edit.error = None;
                    cx.notify();
                }
            }
            _ => {}
        });
        self.editing = Some(Edit {
            kind,
            dir,
            target,
            input: input.clone(),
            error: None,
            _sub: sub,
        });
        cx.notify();
        // 描いた後でフォーカスし、名前を拡張子の手前まで選ぶ（`.gitignore` のような名前は全部）。
        let stem_end = match initial.rfind('.') {
            Some(i) if i > 0 => i,
            _ => initial.len(),
        };
        window.defer(cx, move |window, cx| {
            input.update(cx, |s, cx| {
                s.focus(window, cx);
                s.set_selected_range(0..stem_end, cx);
            });
        });
    }

    /// 入力を確定する。`on_blur` のとき、空・変わっていない・正しくない名前なら黙って取り消す。
    pub(crate) fn commit_edit(&mut self, on_blur: bool, cx: &mut Context<Self>) {
        let Some(edit) = self.editing.as_ref() else { return };
        let name = edit.input.read(cx).value().trim().to_string();
        let (kind, dir, target) = (edit.kind, edit.dir.clone(), edit.target.clone());
        let unchanged = target
            .as_ref()
            .and_then(|t| t.file_name())
            .is_some_and(|n| n.to_string_lossy() == name);
        if name.is_empty() || unchanged {
            self.cancel_edit(cx);
            return;
        }
        let existing = self
            .tree
            .as_ref()
            .and_then(|t| t.entry(&dir.join(&name)))
            .map(|e| e.kind);
        // 大文字小文字だけを変える名前の変更は、同じものとして許す。
        let case_only = target
            .as_ref()
            .and_then(|t| t.file_name())
            .is_some_and(|n| n.to_string_lossy().to_lowercase() == name.to_lowercase());
        if !valid_name(&name) {
            return self.edit_failed(on_blur, cx.t("core.tree.invalid-name").to_string(), cx);
        }
        if let Some(kind) = existing.filter(|_| !case_only) {
            return self.edit_failed(on_blur, self.exists_message(&dir, &name, kind, cx), cx);
        }
        let new = dir.join(&name);
        let client = cx.remote();
        type Op = std::pin::Pin<Box<dyn std::future::Future<Output = Result<(), remote::RemoteError>>>>;
        let op: Op = match kind {
            EditKind::NewFolder => Box::pin(client.create_dir(&new)),
            EditKind::NewFile => Box::pin(client.create_file(&new)),
            EditKind::Rename => match &target {
                Some(old) => Box::pin(client.rename(old, &new)),
                None => return self.cancel_edit(cx),
            },
        };
        cx.spawn(async move |this, cx| {
            let result = op.await;
            this.update(cx, |this, cx| match result {
                Ok(()) => {
                    match (kind, &target) {
                        (EditKind::NewFile, _) => cx.emit(ProjectPanelEvent::OpenFile(new.clone())),
                        (EditKind::Rename, Some(old)) => cx.emit(ProjectPanelEvent::Renamed(old.clone(), new.clone())),
                        _ => {}
                    }
                    this.editing = None;
                    this.refresh_dir(&dir, cx);
                    this.selected = Some(new.clone());
                    cx.notify();
                }
                // ツリーがまだ知らないものと名前がぶつかった（ほかで作られたばかりなど）。
                Err(e) if e.kind == remote::ErrorKind::AlreadyExists => {
                    let msg = this.exists_message(&dir, &name, EntryKind::File, cx);
                    this.edit_failed(on_blur, msg, cx);
                }
                Err(e) => this.edit_failed(on_blur, e.to_string(), cx),
            })
            .ok();
        })
        .detach();
    }

    /// どこに、何（ファイルかフォルダか）があるのかを言う（VSCode と同じ）。
    fn exists_message(&self, dir: &Path, name: &str, kind: EntryKind, cx: &App) -> String {
        let key = match kind {
            EntryKind::Dir => "core.tree.exists-folder",
            EntryKind::File => "core.tree.exists-file",
        };
        let place = dir
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| dir.display().to_string());
        cx.t_args(key, &[("name", name.to_string().into()), ("dir", place.into())])
            .to_string()
    }

    /// 入力の下にエラーを出す。ほかを押して離れたときは、エラーを出したまま残さず取り消す。
    fn edit_failed(&mut self, on_blur: bool, message: String, cx: &mut Context<Self>) {
        if on_blur {
            return self.cancel_edit(cx);
        }
        if let Some(edit) = self.editing.as_mut() {
            edit.error = Some(message);
        }
        cx.notify();
    }

    pub(crate) fn cancel_edit(&mut self, cx: &mut Context<Self>) {
        if self.editing.take().is_some() {
            cx.notify();
        }
    }
}

/// ツリーの中で入力している名前。
pub(crate) struct Edit {
    pub kind: EditKind,
    /// 作る（変える）場所のフォルダ。
    pub dir: PathBuf,
    /// 名前を変えるもの。
    pub target: Option<PathBuf>,
    pub input: Entity<InputState>,
    pub error: Option<String>,
    _sub: gpui_kit::Subscription,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum EditKind {
    NewFile,
    NewFolder,
    Rename,
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
