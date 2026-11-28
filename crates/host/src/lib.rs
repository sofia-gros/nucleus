//! ホスト: ファイル・プロセス・拡張機能などを実際に扱う側。
//!
//! 画面とは `remote` の取り決め（[`remote::proto`]）で話す。画面 1 つにつき [`Connection`] が 1 つ。
//! デスクトップ版は同じプロセスの中で [`local`] を使ってつなぎ、サーバー版はネットワーク越しにつなぐ。
//! どちらも同じ符号化を通るので、動きは変わらない。

pub mod fs;
pub mod search;
mod watcher;

use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    sync::{
        Arc, Mutex, OnceLock,
        atomic::{AtomicBool, Ordering},
    },
};

use remote::{
    ErrorKind, Event, RemoteError, Request, Response,
    proto::{self, Frame},
};

pub use watcher::DirWatcher;

/// すべての接続で共有するもの（共同開発では、ここにみんなが同じものを見る）。
#[derive(Clone, Default)]
pub struct Host {
    _state: Arc<HostState>,
}

#[derive(Default)]
struct HostState {}

impl Host {
    pub fn new() -> Host {
        Host::default()
    }

    /// 新しい画面をつなぐ。`send` は画面へフレームを届ける。
    pub fn connect(&self, send: remote::Sender) -> Connection {
        Connection(Arc::new(ConnInner {
            _host: self.clone(),
            send,
            cancels: Mutex::new(HashMap::new()),
            watcher: Mutex::new(None),
            closed: AtomicBool::new(false),
        }))
    }
}

/// 画面 1 つとのつながり。
#[derive(Clone)]
pub struct Connection(Arc<ConnInner>);

struct ConnInner {
    _host: Host,
    send: remote::Sender,
    /// 打ち切れる頼みごと（検索など）。
    cancels: Mutex<HashMap<u64, Arc<AtomicBool>>>,
    watcher: Mutex<Option<DirWatcher>>,
    closed: AtomicBool,
}

impl Connection {
    /// 画面から届いたフレーム。重い仕事は別のスレッドで行い、すぐ戻る。
    pub fn receive(&self, bytes: &[u8]) {
        match proto::decode(bytes) {
            Ok(Frame::Request { id, request }) => self.handle(id, request),
            Ok(Frame::Cancel { id }) => {
                if let Some(flag) = self.0.cancels.lock().unwrap().remove(&id) {
                    flag.store(true, Ordering::Relaxed);
                }
            }
            Ok(other) => log::warn!("画面から届くはずのないフレーム: {other:?}"),
            Err(e) => log::warn!("画面からのフレームを読めない: {e}"),
        }
    }

    /// 画面が去った。見張りなどを止める。
    pub fn close(&self) {
        self.0.closed.store(true, Ordering::SeqCst);
        self.0.watcher.lock().unwrap().take();
        for (_, flag) in self.0.cancels.lock().unwrap().drain() {
            flag.store(true, Ordering::Relaxed);
        }
    }

    fn send(&self, frame: &Frame) {
        if !self.0.closed.load(Ordering::SeqCst) {
            (self.0.send)(proto::encode(frame));
        }
    }

    fn reply(&self, id: u64, result: Result<Response, RemoteError>) {
        self.send(&Frame::Response { id, result });
    }

    fn event(&self, event: Event) {
        self.send(&Frame::Event(event));
    }

    fn handle(&self, id: u64, request: Request) {
        // すぐ終わるものはその場で。
        match request {
            Request::Watch { dir } => {
                self.watch(Path::new(&dir));
                return self.reply(id, Ok(Response::Done));
            }
            Request::Unwatch { dir } => {
                if let Some(w) = self.0.watcher.lock().unwrap().as_mut() {
                    w.unwatch(Path::new(&dir));
                }
                return self.reply(id, Ok(Response::Done));
            }
            _ => {}
        }
        let cancel = Arc::new(AtomicBool::new(false));
        self.0.cancels.lock().unwrap().insert(id, cancel.clone());
        let this = self.clone();
        let spawned = std::thread::Builder::new().name("host-request".into()).spawn(move || {
            let result = run(request, &cancel);
            this.0.cancels.lock().unwrap().remove(&id);
            if !cancel.load(Ordering::Relaxed) {
                this.reply(id, result);
            }
        });
        if let Err(e) = spawned {
            self.reply(id, Err(RemoteError::other(e.to_string())));
        }
    }

    fn watch(&self, dir: &Path) {
        // まだないフォルダ（.nucleus など）は見張れない。できたら、親の変更で読み直される。
        if !dir.is_dir() {
            return;
        }
        let mut slot = self.0.watcher.lock().unwrap();
        if slot.is_none() {
            match DirWatcher::new() {
                Ok((w, rx)) => {
                    *slot = Some(w);
                    let this = self.clone();
                    std::thread::Builder::new()
                        .name("host-watch".into())
                        .spawn(move || {
                            for dir in futures::executor::block_on_stream(rx) {
                                this.event(Event::DirChanged(remote::wire(&dir)));
                            }
                        })
                        .ok();
                }
                Err(e) => {
                    log::warn!("ファイルの見張りを始められない: {e}");
                    return;
                }
            }
        }
        if let Some(w) = slot.as_mut() {
            w.watch(dir);
        }
    }
}

/// 時間のかかる頼みごとを実行する（別のスレッドで呼ぶ）。
fn run(request: Request, cancel: &AtomicBool) -> Result<Response, RemoteError> {
    let p = PathBuf::from;
    Ok(match request {
        Request::ReadFile { path, max_size } => match fs::read_limited(&p(path), max_size) {
            Ok(bytes) => Response::Bytes(bytes),
            Err(Ok(size)) => {
                return Err(RemoteError::new(
                    ErrorKind::TooLarge(size),
                    format!("file is too large ({size} bytes)"),
                ));
            }
            Err(Err(e)) => return Err(e.into()),
        },
        Request::WriteFile { path, bytes } => {
            fs::write_atomic(&p(path), &bytes)?;
            Response::Done
        }
        Request::ReadDir { path } => Response::Entries(fs::read_dir(&p(path))?),
        Request::CreateFile { path } => {
            fs::create_file(&p(path))?;
            Response::Done
        }
        Request::CreateDir { path } => {
            std::fs::create_dir_all(p(path))?;
            Response::Done
        }
        Request::Rename { from, to } => {
            fs::rename(&p(from), &p(to))?;
            Response::Done
        }
        Request::Trash { path } => {
            fs::trash(&p(path))?;
            Response::Done
        }
        Request::Exists { path } => Response::Bool(p(path).exists()),
        Request::ListFiles { root } => Response::Paths(fs::list_files(&p(root))),
        Request::Search { root, options, limit } => match search::search(&p(root), &options, limit, cancel) {
            Ok(r) => Response::Search(r),
            Err(e) if cancel.load(Ordering::Relaxed) => return Err(RemoteError::new(ErrorKind::Cancelled, e)),
            Err(e) => return Err(RemoteError::other(e)),
        },
        Request::Watch { .. } | Request::Unwatch { .. } => Response::Done,
    })
}

/// 同じプロセスの中でホストと画面をつなぐ（デスクトップ版）。
pub fn local(host: &Host) -> remote::Client {
    let conn: Arc<OnceLock<Connection>> = Arc::new(OnceLock::new());
    let to_host = conn.clone();
    let client = remote::Client::new(Box::new(move |bytes| {
        if let Some(c) = to_host.get() {
            c.receive(&bytes);
        }
    }));
    let to_client = client.clone();
    let _ = conn.set(host.connect(Box::new(move |bytes| to_client.receive(&bytes))));
    client
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures::executor::block_on;

    #[test]
    fn local_client_reaches_the_file_system() {
        let d = tempfile::tempdir().unwrap();
        let client = local(&Host::new());
        let a = d.path().join("a.txt");
        block_on(client.create_file(&a)).unwrap();
        assert_eq!(
            block_on(client.create_file(&a)).unwrap_err().kind,
            ErrorKind::AlreadyExists
        );
        block_on(client.write_file(&a, b"hello".to_vec())).unwrap();
        assert_eq!(block_on(client.read_file(&a, 100)).unwrap(), b"hello");
        assert_eq!(
            block_on(client.read_file(&a, 2)).unwrap_err().kind,
            ErrorKind::TooLarge(5)
        );
        let entries = block_on(client.read_dir(d.path())).unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(
            block_on(client.read_file(&d.path().join("missing"), 9))
                .unwrap_err()
                .kind,
            ErrorKind::NotFound
        );
    }

    #[test]
    fn watched_dirs_report_changes() {
        let d = tempfile::tempdir().unwrap();
        let client = local(&Host::new());
        let mut events = client.subscribe();
        client.watch(d.path());
        // 見張りが始まるのを待ってから書く。
        std::thread::sleep(std::time::Duration::from_millis(200));
        std::fs::write(d.path().join("x"), "").unwrap();
        let got = block_on(async {
            use futures::StreamExt as _;
            events.next().await
        });
        let Some(Event::DirChanged(dir)) = got else {
            panic!("{got:?}")
        };
        assert_eq!(
            PathBuf::from(dir).canonicalize().unwrap(),
            d.path().canonicalize().unwrap()
        );
    }
}
