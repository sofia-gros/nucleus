//! 行番号の横の変更の印の「元の内容」を、Extension の `contributes.quick_diff` に従って取ってくる。

use std::path::Path;

use editor::EditorPanel;
use gpui_kit::{AppContext as _, Context, Entity, WeakEntity};

use crate::Workspace;

impl Workspace {
    /// 開いているすべてのエディタの元の内容を取り直す（ウィンドウに戻ったとき。コミットなどで変わるため）。
    pub(crate) fn refresh_all_diff_bases(&mut self, cx: &mut Context<Self>) {
        let editors: Vec<_> = self.editors.values().filter_map(WeakEntity::upgrade).collect();
        for e in editors {
            self.refresh_diff_base(&e, cx);
        }
    }

    /// 1つのエディタの元の内容を取ってくる（開いたとき・保存したとき）。
    pub(crate) fn refresh_diff_base(&mut self, editor: &Entity<EditorPanel>, cx: &mut Context<Self>) {
        let Some(store) = extension_host::ExtensionStore::get(cx) else {
            return;
        };
        let Some(q) = store.enabled().find_map(|e| e.manifest.contributes.quick_diff.clone()) else {
            return;
        };
        let Some(root) = self.project.read(cx).root() else {
            return;
        };
        let path = editor.read(cx).path().to_path_buf();
        let Some(rel) = relative(&root, &path) else { return };
        let args: Vec<String> = q.args.iter().map(|a| a.replace("${path}", &rel)).collect();
        let weak = editor.downgrade();
        let task = cx.background_spawn(async move {
            let out = std::process::Command::new(&q.command)
                .args(&args)
                .current_dir(&root)
                .env("GIT_TERMINAL_PROMPT", "0")
                .stdin(std::process::Stdio::null())
                .output()
                .ok()?;
            // 失敗（まだ追跡していないファイルなど）なら印は出さない。
            out.status
                .success()
                .then(|| String::from_utf8_lossy(&out.stdout).into_owned())
        });
        cx.spawn(async move |_, cx| {
            let base = task.await;
            weak.update(cx, |e, cx| e.set_diff_base(base, cx)).ok();
        })
        .detach();
    }
}

fn relative(root: &Path, path: &Path) -> Option<String> {
    let rel = path.strip_prefix(root).ok()?;
    Some(rel.to_string_lossy().replace('\\', "/"))
}
