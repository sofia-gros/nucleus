use std::{
    collections::HashMap,
    future::Future,
    path::PathBuf,
    pin::Pin,
    sync::{
        Arc, Mutex,
        atomic::{AtomicI64, Ordering},
    },
};

use anyhow::anyhow;
use futures::{
    StreamExt as _,
    channel::{mpsc::UnboundedSender, oneshot},
};
use lsp_types::{
    ClientCapabilities, InitializeParams, InitializeResult, PositionEncodingKind, ServerCapabilities, Uri,
    WorkspaceFolder, notification::Notification, request::Request,
};
use serde_json::{Value, json};

/// 言語サーバーとのつながり。サーバーの起動と読み書きは、ほか（ホスト）が受け持つ。
pub struct Link {
    /// メッセージ（JSON）を 1 つ送る。
    pub send: Box<dyn Fn(Vec<u8>) + Send + Sync>,
    /// サーバーからのメッセージ（JSON）。サーバーが終わると閉じる。
    pub incoming: futures::stream::BoxStream<'static, Vec<u8>>,
    /// サーバーを止める。
    pub stop: Box<dyn Fn() + Send + Sync>,
}

/// 読み込みの Future を動かしてもらうための入れ物（実行のしかたは呼び出し側が決める）。
pub type Spawn = Box<dyn FnOnce(Pin<Box<dyn Future<Output = ()> + Send>>)>;

/// 言語サーバーの起動方法（Extension の `contributes.language_servers`）。
#[derive(Clone, Debug, PartialEq)]
pub struct ServerConfig {
    pub name: String,
    pub command: String,
    pub args: Vec<String>,
    pub initialization_options: Option<Value>,
}

/// サーバーから届いた通知。
#[derive(Debug)]
pub struct ServerNotification {
    pub method: String,
    pub params: Value,
}

type Pending = Arc<Mutex<HashMap<i64, oneshot::Sender<Result<Value, String>>>>>;

pub struct LanguageServer {
    pub config: ServerConfig,
    pub capabilities: ServerCapabilities,
    /// 位置の数え方。`utf-32` ならコードポイント（gpui-kit のエディタと同じ）。
    pub position_encoding: PositionEncodingKind,
    send: Arc<dyn Fn(Vec<u8>) + Send + Sync>,
    stop: Box<dyn Fn() + Send + Sync>,
    pending: Pending,
    next_id: AtomicI64,
}

impl LanguageServer {
    /// つながったサーバーに initialize まで済ませる。`spawn` は読み込みの Future を動かす。
    pub async fn connect(
        config: ServerConfig,
        root: PathBuf,
        link: Link,
        notifications: UnboundedSender<ServerNotification>,
        spawn: Spawn,
    ) -> anyhow::Result<Arc<LanguageServer>> {
        let Link {
            send,
            mut incoming,
            stop,
        } = link;
        let send: Arc<dyn Fn(Vec<u8>) + Send + Sync> = Arc::from(send);
        let pending: Pending = Arc::default();
        // 読み込み: 応答は待っている要求へ、通知はチャネルへ、サーバーからの要求には既定の応答を返す。
        {
            let pending = pending.clone();
            let send = send.clone();
            let name = config.name.clone();
            spawn(Box::pin(async move {
                while let Some(body) = incoming.next().await {
                    let Ok(msg) = serde_json::from_slice::<Value>(&body) else {
                        continue;
                    };
                    let method = msg.get("method").and_then(Value::as_str).map(str::to_string);
                    let id = msg.get("id").cloned();
                    match (method, id) {
                        (None, Some(id)) => {
                            let Some(id) = id.as_i64() else { continue };
                            let result = match msg.get("error") {
                                Some(e) => Err(e.get("message").and_then(Value::as_str).unwrap_or("error").to_string()),
                                None => Ok(msg.get("result").cloned().unwrap_or(Value::Null)),
                            };
                            if let Some(tx) = pending.lock().expect("pending").remove(&id) {
                                let _ = tx.send(result);
                            }
                        }
                        (Some(method), Some(id)) => {
                            let result = default_response(&method, msg.get("params"));
                            let reply = json!({ "jsonrpc": "2.0", "id": id, "result": result });
                            send(serde_json::to_vec(&reply).expect("json"));
                        }
                        (Some(method), None) => {
                            let params = msg.get("params").cloned().unwrap_or(Value::Null);
                            let _ = notifications.unbounded_send(ServerNotification { method, params });
                        }
                        _ => {}
                    }
                }
                log::info!("言語サーバー `{name}` の出力が終わった");
                // 残っている要求はすべて失敗にする。
                for (_, tx) in pending.lock().expect("pending").drain() {
                    let _ = tx.send(Err("server exited".into()));
                }
            }));
        }

        let mut server = LanguageServer {
            config,
            capabilities: ServerCapabilities::default(),
            position_encoding: PositionEncodingKind::UTF16,
            send,
            stop,
            pending,
            next_id: AtomicI64::new(1),
        };
        let root_uri = path_to_uri(&root)?;
        let name = root
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        #[allow(deprecated)]
        let params = InitializeParams {
            // サーバーはホストが動かし、止めるのもホストなので、こちらのプロセス番号は伝えない。
            process_id: None,
            root_uri: Some(root_uri.clone()),
            workspace_folders: Some(vec![WorkspaceFolder { uri: root_uri, name }]),
            initialization_options: server.config.initialization_options.clone(),
            capabilities: client_capabilities(),
            client_info: Some(lsp_types::ClientInfo {
                name: "Nucleus".into(),
                version: None,
            }),
            ..Default::default()
        };
        let result: InitializeResult = server.request_raw::<lsp_types::request::Initialize>(params).await?;
        server.position_encoding = result
            .capabilities
            .position_encoding
            .clone()
            .unwrap_or(PositionEncodingKind::UTF16);
        server.capabilities = result.capabilities;
        server.notify::<lsp_types::notification::Initialized>(lsp_types::InitializedParams {});
        Ok(Arc::new(server))
    }

    async fn request_raw<R: Request>(&self, params: R::Params) -> anyhow::Result<R::Result> {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);
        let (tx, rx) = oneshot::channel();
        self.pending.lock().expect("pending").insert(id, tx);
        let msg = json!({ "jsonrpc": "2.0", "id": id, "method": R::METHOD, "params": params });
        (self.send)(serde_json::to_vec(&msg)?);
        let value = rx
            .await
            .map_err(|_| anyhow!("応答がない"))?
            .map_err(|e| anyhow!("{}: {e}", R::METHOD))?;
        Ok(serde_json::from_value(value)?)
    }

    /// 要求を送り、応答を待つ（Future はバックグラウンドで await すること）。
    pub async fn request<R: Request>(&self, params: R::Params) -> anyhow::Result<R::Result> {
        self.request_raw::<R>(params).await
    }

    pub fn notify<N: Notification>(&self, params: N::Params) {
        let msg = json!({ "jsonrpc": "2.0", "method": N::METHOD, "params": params });
        if let Ok(body) = serde_json::to_vec(&msg) {
            (self.send)(body);
        }
    }

    /// shutdown → exit。終わらなければ止める。
    pub async fn shutdown(&self) {
        let _ = self.request_raw::<lsp_types::request::Shutdown>(()).await;
        self.notify::<lsp_types::notification::Exit>(());
        (self.stop)();
    }

    pub fn uses_utf32(&self) -> bool {
        self.position_encoding == PositionEncodingKind::UTF32
    }
}

impl Drop for LanguageServer {
    fn drop(&mut self) {
        (self.stop)();
    }
}

/// サーバーからの要求に返す既定の応答。
fn default_response(method: &str, params: Option<&Value>) -> Value {
    match method {
        // 設定は持っていないので、要求された数だけ null を返す。
        "workspace/configuration" => {
            let n = params
                .and_then(|p| p.get("items"))
                .and_then(Value::as_array)
                .map_or(0, Vec::len);
            Value::Array(vec![Value::Null; n])
        }
        _ => Value::Null,
    }
}

fn client_capabilities() -> ClientCapabilities {
    serde_json::from_value(json!({
        "general": { "positionEncodings": ["utf-32", "utf-16"] },
        "textDocument": {
            "synchronization": { "didSave": true, "dynamicRegistration": false },
            "completion": {
                "completionItem": { "snippetSupport": false, "documentationFormat": ["markdown", "plaintext"] },
                "contextSupport": true
            },
            "hover": { "contentFormat": ["markdown", "plaintext"] },
            "definition": { "linkSupport": true },
            "publishDiagnostics": { "relatedInformation": false }
        },
        "window": { "workDoneProgress": true },
        "workspace": { "configuration": true, "workspaceFolders": true }
    }))
    .expect("valid capabilities")
}

pub fn path_to_uri(path: &std::path::Path) -> anyhow::Result<Uri> {
    let s = path.to_string_lossy().replace('\\', "/");
    let encoded: String = s
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || "/-_.~".contains(c) {
                c.to_string()
            } else {
                let mut buf = [0u8; 4];
                c.encode_utf8(&mut buf).bytes().map(|b| format!("%{b:02X}")).collect()
            }
        })
        .collect();
    let prefix = if encoded.starts_with('/') {
        "file://"
    } else {
        "file:///"
    };
    format!("{prefix}{encoded}")
        .parse()
        .map_err(|e| anyhow!("URI にできない: {e:?}"))
}

pub fn uri_to_path(uri: &Uri) -> Option<PathBuf> {
    let s = uri.as_str().strip_prefix("file://")?;
    let mut bytes = Vec::new();
    let mut it = s.bytes();
    while let Some(b) = it.next() {
        if b == b'%' {
            let h = it.next()?;
            let l = it.next()?;
            bytes.push(u8::from_str_radix(&format!("{}{}", h as char, l as char), 16).ok()?);
        } else {
            bytes.push(b);
        }
    }
    let p = String::from_utf8(bytes).ok()?;
    // Windows: /C:/x → C:/x
    let p = if p.len() > 2 && p.as_bytes()[2] == b':' {
        p[1..].to_string()
    } else {
        p
    };
    Some(PathBuf::from(p))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn uris_round_trip() {
        let p = PathBuf::from("/tmp/a b/日本.rs");
        let uri = path_to_uri(&p).unwrap();
        assert_eq!(uri.as_str(), "file:///tmp/a%20b/%E6%97%A5%E6%9C%AC.rs");
        assert_eq!(uri_to_path(&uri).unwrap(), p);
    }
}
