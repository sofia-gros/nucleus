//! 言語サーバーを動かす。標準入出力の読み書きは専用のスレッドで行い、メッセージ（JSON）をそのまま中継する。
//! 要求と応答の対応付けは画面側（lsp クレート）が行う。

use std::{
    io::BufReader,
    process::{Child, Stdio},
    sync::{Arc, Mutex, mpsc},
};

use remote::Event;

use crate::lsp_transport as transport;

pub struct HostServer {
    writer: mpsc::Sender<Vec<u8>>,
    child: Arc<Mutex<Child>>,
}

impl HostServer {
    /// 起動する。`emit` はサーバーからのメッセージと終了を画面側へ届ける。
    pub fn start(
        id: u64,
        command: &str,
        args: &[String],
        cwd: &str,
        emit: impl Fn(Event) + Send + 'static,
    ) -> std::io::Result<HostServer> {
        let mut cmd = background::command(command);
        cmd.args(args)
            .current_dir(cwd)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        let mut child = cmd.spawn()?;
        let missing = || std::io::Error::other("pipe missing");
        let mut stdin = child.stdin.take().ok_or_else(missing)?;
        let stdout = child.stdout.take().ok_or_else(missing)?;
        let stderr = child.stderr.take();

        let (writer, rx) = mpsc::channel::<Vec<u8>>();
        std::thread::Builder::new()
            .name(format!("lsp-write {command}"))
            .spawn(move || {
                while let Ok(body) = rx.recv() {
                    if transport::write_message(&mut stdin, &body).is_err() {
                        break;
                    }
                }
            })?;
        let name = command.to_string();
        std::thread::Builder::new()
            .name(format!("lsp-read {command}"))
            .spawn(move || {
                let mut reader = BufReader::new(stdout);
                while let Ok(Some(body)) = transport::read_message(&mut reader) {
                    emit(Event::LspMessage { id, body });
                }
                log::info!("言語サーバー `{name}` の出力が終わった");
                emit(Event::LspExited { id });
            })?;
        if let Some(stderr) = stderr {
            let name = command.to_string();
            std::thread::Builder::new()
                .name(format!("lsp-err {command}"))
                .spawn(move || {
                    use std::io::BufRead as _;
                    for line in BufReader::new(stderr).lines().map_while(Result::ok) {
                        log::debug!("[{name}] {line}");
                    }
                })?;
        }
        Ok(HostServer {
            writer,
            child: Arc::new(Mutex::new(child)),
        })
    }

    pub fn send(&self, body: Vec<u8>) {
        let _ = self.writer.send(body);
    }
}

impl Drop for HostServer {
    fn drop(&mut self) {
        // shutdown と exit は画面側が送る。それでも残っていれば止める（少し待ってから）。
        let child = self.child.clone();
        std::thread::spawn(move || {
            std::thread::sleep(std::time::Duration::from_secs(2));
            if let Ok(mut c) = child.lock() {
                let _ = c.kill();
                let _ = c.wait();
            }
        });
    }
}
