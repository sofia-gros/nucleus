//! 組み込みの Word Tools（WASM）を実際に動かして、ホスト関数と権限の確認を試す。

use std::sync::{Arc, Mutex};

use extension_host::{ActiveDocument, HostContext, Manifest, NoticeLevel, UiRequest, runtime};
use futures::{StreamExt, executor::block_on};

const WASM: &[u8] = include_bytes!("../../../extensions/nucleus.word-tools/word_tools.wasm");
const MANIFEST: &str = include_str!("../../../extensions/nucleus.word-tools/extension.yaml");

#[derive(Default)]
struct Ui {
    notices: Vec<(NoticeLevel, String)>,
    replaced: Option<String>,
}

/// UI スレッドの代わり。
fn fake_ui(rx: futures::channel::mpsc::UnboundedReceiver<UiRequest>) -> Arc<Mutex<Ui>> {
    let ui = Arc::new(Mutex::new(Ui::default()));
    let out = ui.clone();
    std::thread::spawn(move || {
        let mut rx = rx;
        while let Some(req) = block_on(rx.next()) {
            match req {
                UiRequest::Notify { level, message, .. } => ui.lock().unwrap().notices.push((level, message)),
                UiRequest::ActiveDocument { reply } => {
                    let _ = reply.send(Some(ActiveDocument {
                        path: "notes.txt".into(),
                        language: "text".into(),
                        text: "pear apple\nbanana\n".into(),
                    }));
                }
                UiRequest::ReplaceActiveText { text, reply } => {
                    ui.lock().unwrap().replaced = Some(text);
                    let _ = reply.send(Ok(()));
                }
                UiRequest::OpenFile { .. } | UiRequest::Invalidate { .. } => {}
            }
        }
    });
    out
}

fn start(manifest: Manifest) -> (runtime::WasmExtension, Arc<Mutex<Ui>>) {
    let (host, rx) = HostContext::new();
    let ui = fake_ui(rx);
    let (ext, ready) = runtime::spawn(manifest, Arc::from(WASM), host);
    block_on(ready).unwrap().expect("activate");
    (ext, ui)
}

#[test]
fn word_tools_counts_and_sorts() {
    let manifest = Manifest::from_yaml(MANIFEST).unwrap();
    assert!(manifest.validate().is_empty(), "{:?}", manifest.validate());
    let (ext, ui) = start(manifest);

    block_on(ext.run_command("word-tools.count")).unwrap().unwrap();
    // notify は返事を待たないので、届くまで少し待つ。
    for _ in 0..100 {
        if !ui.lock().unwrap().notices.is_empty() {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    let notices = ui.lock().unwrap().notices.clone();
    assert_eq!(
        notices,
        vec![(NoticeLevel::Info, "notes.txt: 3 words, 2 lines, 18 characters".into())]
    );

    block_on(ext.run_command("word-tools.sort-lines")).unwrap().unwrap();
    assert_eq!(ui.lock().unwrap().replaced.as_deref(), Some("banana\npear apple\n"));

    let err = block_on(ext.run_command("word-tools.nope")).unwrap().unwrap_err();
    assert!(err.contains("unknown command"), "{err}");
}

#[test]
fn host_functions_check_permissions() {
    let mut manifest = Manifest::from_yaml(MANIFEST).unwrap();
    manifest.permissions.insert("editor".into(), vec!["read".into()]);
    let (ext, ui) = start(manifest);
    let err = block_on(ext.run_command("word-tools.sort-lines")).unwrap().unwrap_err();
    assert!(err.contains("editor: [edit]"), "{err}");
    assert!(ui.lock().unwrap().replaced.is_none());
}

const GIT_WASM: &[u8] = include_bytes!("../../../extensions/nucleus.git/git.wasm");
const GIT_MANIFEST: &str = include_str!("../../../extensions/nucleus.git/extension.yaml");

fn texts(nodes: &[extension_host::ui::Node]) -> Vec<String> {
    use extension_host::ui::Kind;
    nodes
        .iter()
        .filter_map(|n| match &n.kind {
            Kind::Text(t) => Some(t.content.clone()),
            Kind::ListItem(l) => Some(l.label.clone()),
            Kind::Section(s) => Some(s.title.clone()),
            Kind::Button(b) => b.label.clone(),
            _ => None,
        })
        .collect()
}

/// ui@1: Git Extension が要素の木を返し、イベントで状態が変わる。
#[test]
fn git_view_renders_and_stages() {
    if std::process::Command::new("git").arg("--version").output().is_err() {
        eprintln!("git がないので飛ばす");
        return;
    }
    let d = tempfile::tempdir().unwrap();
    let git = |args: &[&str]| {
        let ok = std::process::Command::new("git")
            .args(args)
            .current_dir(d.path())
            .output()
            .unwrap()
            .status
            .success();
        assert!(ok, "git {args:?}");
    };
    git(&["init", "-q", "-b", "main"]);
    std::fs::write(d.path().join("a.txt"), "hello\n").unwrap();

    let manifest = Manifest::from_yaml(GIT_MANIFEST).unwrap();
    assert!(manifest.validate().is_empty(), "{:?}", manifest.validate());
    let (host, rx) = HostContext::new();
    host.set_workspace_root(Some(d.path().to_path_buf()));
    let _ui = fake_ui_any(rx);
    let (ext, ready) = runtime::spawn(manifest, Arc::from(GIT_WASM), host);
    block_on(ready).unwrap().expect("activate");

    let nodes = block_on(ext.render("changes")).unwrap().unwrap();
    runtime::validate_tree(&nodes).unwrap();
    let t = texts(&nodes);
    assert!(t.contains(&"Unstaged Changes".to_string()), "{t:?}");
    assert!(t.contains(&"a.txt".to_string()), "{t:?}");

    block_on(ext.ui_event("changes", "stage-all", extension_host::ui::UiEvent::Click))
        .unwrap()
        .unwrap();
    let nodes = block_on(ext.render("changes")).unwrap().unwrap();
    let staged = nodes.iter().any(|n| {
        matches!(&n.kind, extension_host::ui::Kind::Section(s) if s.title == "Staged Changes" && s.count == Some(1))
    });
    assert!(staged, "{:?}", texts(&nodes));
}

/// 返事だけする UI（通知や描き直しの依頼は捨てる）。
fn fake_ui_any(rx: futures::channel::mpsc::UnboundedReceiver<UiRequest>) -> std::thread::JoinHandle<()> {
    std::thread::spawn(move || {
        let mut rx = rx;
        while let Some(req) = block_on(rx.next()) {
            match req {
                UiRequest::ActiveDocument { reply } => {
                    let _ = reply.send(None);
                }
                UiRequest::ReplaceActiveText { reply, .. } => {
                    let _ = reply.send(Err("none".into()));
                }
                _ => {}
            }
        }
    })
}
