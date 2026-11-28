//! 画面とホストのあいだの取り決め（[`proto`]）と、画面側の窓口（[`Client`]）。
//!
//! 画面（workspace など）は、ファイルやプロセスに直接さわらず、すべて [`Client`] を通してホストに頼む。
//! ホストが同じ PC の中にいても（デスクトップ版）、ネットワークの向こうにいても（ブラウザ版・リモート接続）、
//! 画面のコードは同じになる。

#[cfg(feature = "gpui")]
mod app;
pub mod proto;
#[cfg(feature = "gpui")]
pub use app::{RemoteExt, set_global};

use std::{
    collections::HashMap,
    future::Future,
    path::{Path, PathBuf},
    pin::Pin,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, AtomicU64, Ordering},
    },
    task::{Context, Poll},
};

use futures::channel::{mpsc, oneshot};

use project::{Entry, search::SearchOptions, search::SearchResult};
use proto::Frame;
pub use proto::{
    CursorShape, ErrorKind, Event, ProcessOutput, ProcessSpec, RemoteError, Request, Response, TermCell, TermColor,
    TermCursor, TermSize, TermUpdate, TerminalSpec, cell_flags, term_mode,
};

/// 送り出し口。符号化済みのフレームを相手に届ける。
pub type Sender = Box<dyn Fn(Vec<u8>) + Send + Sync>;

/// 知らせの受け手と、受け取るものの選び方。
struct Subscriber {
    filter: Box<dyn Fn(&Event) -> bool + Send + Sync>,
    tx: mpsc::UnboundedSender<Event>,
}

/// ホストへの窓口。複製しても同じつながりを指す。
#[derive(Clone)]
pub struct Client(Arc<Inner>);

struct Inner {
    send: Sender,
    next_id: AtomicU64,
    pending: Mutex<HashMap<u64, oneshot::Sender<Result<Response, RemoteError>>>>,
    subscribers: Mutex<Vec<Subscriber>>,
    closed: AtomicBool,
}

impl Client {
    /// `send` はフレームを相手へ送る。相手からのフレームは [`Client::receive`] に渡す。
    pub fn new(send: Sender) -> Client {
        Client(Arc::new(Inner {
            send,
            next_id: AtomicU64::new(1),
            pending: Mutex::new(HashMap::new()),
            subscribers: Mutex::new(Vec::new()),
            closed: AtomicBool::new(false),
        }))
    }

    /// 相手から届いたフレーム。
    pub fn receive(&self, bytes: &[u8]) {
        match proto::decode(bytes) {
            Ok(Frame::Response { id, result }) => {
                if let Some(tx) = self.0.pending.lock().unwrap().remove(&id) {
                    let _ = tx.send(result);
                }
            }
            Ok(Frame::Event(event)) => {
                self.0
                    .subscribers
                    .lock()
                    .unwrap()
                    .retain(|s| !(s.filter)(&event) || s.tx.unbounded_send(event.clone()).is_ok());
            }
            Ok(other) => log::warn!("ホストから届くはずのないフレーム: {other:?}"),
            Err(e) => log::warn!("ホストからのフレームを読めない: {e}"),
        }
    }

    /// つながりが切れた。待っている頼みごとはすべて [`ErrorKind::Disconnected`] で終わる。
    pub fn disconnected(&self) {
        self.0.closed.store(true, Ordering::SeqCst);
        for (_, tx) in self.0.pending.lock().unwrap().drain() {
            let _ = tx.send(Err(disconnected()));
        }
        self.0.subscribers.lock().unwrap().clear();
    }

    pub fn is_connected(&self) -> bool {
        !self.0.closed.load(Ordering::SeqCst)
    }

    /// ホストからの知らせをすべて受け取る。
    pub fn subscribe(&self) -> mpsc::UnboundedReceiver<Event> {
        self.subscribe_where(|_| true)
    }

    /// ホストからの知らせのうち、`filter` が true を返すものだけを受け取る。
    pub fn subscribe_where(
        &self,
        filter: impl Fn(&Event) -> bool + Send + Sync + 'static,
    ) -> mpsc::UnboundedReceiver<Event> {
        let (tx, rx) = mpsc::unbounded();
        if self.is_connected() {
            self.0.subscribers.lock().unwrap().push(Subscriber {
                filter: Box::new(filter),
                tx,
            });
        }
        rx
    }

    /// 頼みごとを送る。返事を待たずに捨てると、ホストに打ち切りを知らせる。
    pub fn request(&self, request: Request) -> Pending {
        let id = self.0.next_id.fetch_add(1, Ordering::Relaxed);
        let (tx, rx) = oneshot::channel();
        if !self.is_connected() {
            let _ = tx.send(Err(disconnected()));
        } else {
            self.0.pending.lock().unwrap().insert(id, tx);
            (self.0.send)(proto::encode(&Frame::Request { id, request }));
        }
        Pending {
            client: self.clone(),
            id,
            rx,
            done: false,
        }
    }

    fn cancel(&self, id: u64) {
        if self.0.pending.lock().unwrap().remove(&id).is_some() && self.is_connected() {
            (self.0.send)(proto::encode(&Frame::Cancel { id }));
        }
    }

    // ---- 型つきの頼みごと ----

    pub fn read_file(&self, path: &Path, max_size: u64) -> impl Future<Output = Result<Vec<u8>, RemoteError>> + use<> {
        let p = self.request(Request::ReadFile {
            path: wire(path),
            max_size,
        });
        async move {
            match p.await? {
                Response::Bytes(b) => Ok(b),
                r => Err(unexpected(r)),
            }
        }
    }

    pub fn write_file(&self, path: &Path, bytes: Vec<u8>) -> impl Future<Output = Result<(), RemoteError>> + use<> {
        done(self.request(Request::WriteFile {
            path: wire(path),
            bytes,
        }))
    }

    pub fn read_dir(&self, path: &Path) -> impl Future<Output = Result<Vec<Entry>, RemoteError>> + use<> {
        let p = self.request(Request::ReadDir { path: wire(path) });
        async move {
            match p.await? {
                Response::Entries(e) => Ok(e),
                r => Err(unexpected(r)),
            }
        }
    }

    pub fn create_file(&self, path: &Path) -> impl Future<Output = Result<(), RemoteError>> + use<> {
        done(self.request(Request::CreateFile { path: wire(path) }))
    }

    pub fn create_dir(&self, path: &Path) -> impl Future<Output = Result<(), RemoteError>> + use<> {
        done(self.request(Request::CreateDir { path: wire(path) }))
    }

    pub fn rename(&self, from: &Path, to: &Path) -> impl Future<Output = Result<(), RemoteError>> + use<> {
        done(self.request(Request::Rename {
            from: wire(from),
            to: wire(to),
        }))
    }

    pub fn trash(&self, path: &Path) -> impl Future<Output = Result<(), RemoteError>> + use<> {
        done(self.request(Request::Trash { path: wire(path) }))
    }

    pub fn exists(&self, path: &Path) -> impl Future<Output = Result<bool, RemoteError>> + use<> {
        let p = self.request(Request::Exists { path: wire(path) });
        async move {
            match p.await? {
                Response::Bool(b) => Ok(b),
                r => Err(unexpected(r)),
            }
        }
    }

    /// 見張りを頼む（返事は待たない）。
    pub fn watch(&self, dir: &Path) {
        drop(DetachOnDrop(self.request(Request::Watch { dir: wire(dir) })));
    }

    pub fn unwatch(&self, dir: &Path) {
        drop(DetachOnDrop(self.request(Request::Unwatch { dir: wire(dir) })));
    }

    pub fn list_files(&self, root: &Path) -> impl Future<Output = Result<Vec<String>, RemoteError>> + use<> {
        let p = self.request(Request::ListFiles { root: wire(root) });
        async move {
            match p.await? {
                Response::Paths(p) => Ok(p),
                r => Err(unexpected(r)),
            }
        }
    }

    pub fn search(
        &self,
        root: &Path,
        options: SearchOptions,
        limit: usize,
    ) -> impl Future<Output = Result<SearchResult, RemoteError>> + use<> {
        let p = self.request(Request::Search {
            root: wire(root),
            options,
            limit,
        });
        async move {
            match p.await? {
                Response::Search(r) => Ok(r),
                r => Err(unexpected(r)),
            }
        }
    }
}

impl Client {
    /// プログラムを動かして、終わるのを待つ。
    pub fn run_process(&self, spec: ProcessSpec) -> impl Future<Output = Result<ProcessOutput, RemoteError>> + use<> {
        let p = self.request(Request::RunProcess(spec));
        async move {
            match p.await? {
                Response::Output(o) => Ok(o),
                r => Err(unexpected(r)),
            }
        }
    }
}

impl Client {
    /// ターミナルを開く。返事はターミナルの番号。
    pub fn terminal_open(&self, spec: TerminalSpec) -> impl Future<Output = Result<u64, RemoteError>> + use<> {
        let p = self.request(Request::TerminalOpen(spec));
        async move {
            match p.await? {
                Response::Terminal(id) => Ok(id),
                r => Err(unexpected(r)),
            }
        }
    }

    /// 返事を待たずに送る（キー入力など。送った順に届く）。
    pub fn send(&self, request: Request) {
        drop(DetachOnDrop(self.request(request)));
    }
}

/// 返事を待っている頼みごと。
pub struct Pending {
    client: Client,
    id: u64,
    rx: oneshot::Receiver<Result<Response, RemoteError>>,
    done: bool,
}

impl Future for Pending {
    type Output = Result<Response, RemoteError>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        match Pin::new(&mut self.rx).poll(cx) {
            Poll::Ready(r) => {
                self.done = true;
                Poll::Ready(r.unwrap_or_else(|_| Err(disconnected())))
            }
            Poll::Pending => Poll::Pending,
        }
    }
}

impl Drop for Pending {
    fn drop(&mut self) {
        if !self.done {
            self.client.cancel(self.id);
        }
    }
}

/// 返事を気にしない頼みごと（捨てても打ち切らない）。
struct DetachOnDrop(Pending);

impl Drop for DetachOnDrop {
    fn drop(&mut self) {
        self.0.done = true;
        self.0.client.0.pending.lock().unwrap().remove(&self.0.id);
    }
}

async fn done(p: Pending) -> Result<(), RemoteError> {
    match p.await? {
        Response::Done => Ok(()),
        r => Err(unexpected(r)),
    }
}

fn unexpected(r: Response) -> RemoteError {
    RemoteError::new(ErrorKind::Protocol, format!("思っていたのと違う返事: {r:?}"))
}

fn disconnected() -> RemoteError {
    RemoteError::new(ErrorKind::Disconnected, "ホストとのつながりが切れました")
}

/// 送るときのパスの形（区切りは `/`）。
pub fn wire(path: &Path) -> String {
    let s = path.to_string_lossy();
    if cfg!(windows) {
        s.replace('\\', "/")
    } else {
        s.into_owned()
    }
}

/// 届いたパスを、この側の `PathBuf` にする。
pub fn unwire(path: &str) -> PathBuf {
    PathBuf::from(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 返事をすぐ返す、にせのホスト。
    fn echo_client() -> Client {
        let cell: Arc<Mutex<Option<Client>>> = Arc::new(Mutex::new(None));
        let c2 = cell.clone();
        let client = Client::new(Box::new(move |bytes| {
            let Ok(Frame::Request { id, request }) = proto::decode(&bytes) else {
                return;
            };
            let result = match request {
                Request::Exists { path } => Ok(Response::Bool(path == "/yes")),
                _ => Err(RemoteError::other("no")),
            };
            let reply = proto::encode(&Frame::Response { id, result });
            let c = c2.lock().unwrap().clone().unwrap();
            c.receive(&reply);
        }));
        *cell.lock().unwrap() = Some(client.clone());
        client
    }

    #[test]
    fn requests_get_their_own_replies() {
        let c = echo_client();
        let yes = futures::executor::block_on(c.exists(Path::new("/yes")));
        let no = futures::executor::block_on(c.exists(Path::new("/no")));
        assert_eq!((yes, no), (Ok(true), Ok(false)));
        assert!(futures::executor::block_on(c.create_dir(Path::new("/x"))).is_err());
    }

    #[test]
    fn disconnect_fails_pending_and_later_requests() {
        let c = Client::new(Box::new(|_| {}));
        let pending = c.exists(Path::new("/a"));
        c.disconnected();
        let r = futures::executor::block_on(pending);
        assert_eq!(r.unwrap_err().kind, ErrorKind::Disconnected);
        let r = futures::executor::block_on(c.exists(Path::new("/a")));
        assert_eq!(r.unwrap_err().kind, ErrorKind::Disconnected);
    }

    #[test]
    fn events_reach_every_subscriber() {
        let c = Client::new(Box::new(|_| {}));
        let mut a = c.subscribe();
        let mut b = c.subscribe();
        c.receive(&proto::encode(&Frame::Event(Event::DirChanged("/d".into()))));
        for rx in [&mut a, &mut b] {
            let Ok(Event::DirChanged(d)) = rx.try_recv() else {
                panic!()
            };
            assert_eq!(d, "/d");
        }
    }
}
