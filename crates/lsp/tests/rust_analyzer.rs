//! 実物の rust-analyzer との初期化と、診断の通知を確かめる（rust-analyzer がなければスキップ）。

use std::time::Duration;

use futures::StreamExt as _;
use lsp::{LanguageServer, ServerConfig};

#[test]
fn talks_to_rust_analyzer() {
    // rustup の代理コマンドだけがあって本体が入っていないとき（CI など）は、失敗で終わる。
    let installed = std::process::Command::new("rust-analyzer")
        .arg("--version")
        .output()
        .is_ok_and(|o| o.status.success());
    if !installed {
        eprintln!("rust-analyzer がないのでスキップ");
        return;
    }
    let dir = tempfile_dir();
    std::fs::create_dir_all(dir.join("src")).unwrap();
    std::fs::write(
        dir.join("Cargo.toml"),
        "[package]\nname = \"t\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    )
    .unwrap();
    std::fs::write(dir.join("src/main.rs"), "fn main() { let x: i32 = \"no\"; }\n").unwrap();

    futures::executor::block_on(async {
        let (tx, mut rx) = futures::channel::mpsc::unbounded();
        let config = ServerConfig {
            name: "rust-analyzer".into(),
            command: "rust-analyzer".into(),
            args: vec![],
            initialization_options: None,
        };
        let server = LanguageServer::start(config, dir.clone(), tx).await.expect("starts");
        assert!(server.capabilities.completion_provider.is_some());
        assert!(server.uses_utf32(), "rust-analyzer agrees to utf-32");

        let uri = lsp::path_to_uri(&dir.join("src/main.rs")).unwrap();
        server.notify::<lsp::lsp_types::notification::DidOpenTextDocument>(lsp::lsp_types::DidOpenTextDocumentParams {
            text_document: lsp::lsp_types::TextDocumentItem {
                uri: uri.clone(),
                language_id: "rust".into(),
                version: 1,
                text: std::fs::read_to_string(dir.join("src/main.rs")).unwrap(),
            },
        });
        // 診断（型の不一致）が届くまで待つ（最大 60 秒）。
        let deadline = std::time::Instant::now() + Duration::from_secs(60);
        let mut got = false;
        while std::time::Instant::now() < deadline {
            let next = futures::future::select(rx.next(), futures_timer(Duration::from_millis(500))).await;
            if let futures::future::Either::Left((Some(n), _)) = next
                && n.method == "textDocument/publishDiagnostics"
                && n.params["diagnostics"].as_array().is_some_and(|d| !d.is_empty())
            {
                got = true;
                break;
            }
        }
        server.shutdown().await;
        assert!(got, "diagnostics arrive");
    });
}

fn tempfile_dir() -> std::path::PathBuf {
    let d = std::env::temp_dir().join(format!("nucleus-lsp-test-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

fn futures_timer(d: Duration) -> futures::channel::oneshot::Receiver<()> {
    let (tx, rx) = futures::channel::oneshot::channel();
    std::thread::spawn(move || {
        std::thread::sleep(d);
        let _ = tx.send(());
    });
    rx
}
