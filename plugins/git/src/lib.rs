//! Nucleus 公式組み込み Git ソース管理プラグイン (VSCode 完全互換版)

use std::mem;

#[link(wasm_import_module = "env")]
unsafe extern "C" {
    fn host_log(ptr: *const u8, len: i32);
    fn host_invoke(ptr: *const u8, len: i32) -> i64;
}

/// ホストからのメモリ確保要求
#[unsafe(no_mangle)]
pub extern "C" fn alloc(size: usize) -> *mut u8 {
    let mut buf = Vec::with_capacity(size);
    let ptr = buf.as_mut_ptr();
    mem::forget(buf);
    ptr
}

/// ホストからのメモリ解放要求
#[unsafe(no_mangle)]
pub extern "C" fn dealloc(ptr: *mut u8, size: usize) {
    unsafe {
        let _ = Vec::from_raw_parts(ptr, 0, size);
    }
}

/// ホスト API の同期呼び出し
pub fn invoke(api: &str, args: &str) -> String {
    let payload = format!(r#"{{"api": "{}", "args": {}}}"#, api, args);
    let result = unsafe { host_invoke(payload.as_ptr(), payload.len() as i32) };
    
    let ptr = (result >> 32) as *mut u8;
    let len = (result & 0xFFFFFFFF) as usize;
    
    let response = unsafe {
        let slice = std::slice::from_raw_parts(ptr, len);
        String::from_utf8_lossy(slice).into_owned()
    };
    
    dealloc(ptr, len);
    response
}

/// ホストへのログ出力
pub fn log(msg: &str) {
    unsafe {
        host_log(msg.as_ptr(), msg.len() as i32);
    }
}

/// JSON 文字列エスケープ
fn json_escape(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
}

/// ワークスペースのルートパスを取得
fn get_cwd_arg() -> String {
    let root_res = invoke("workspace.get_root_path", "{}");
    if let Some(pos) = root_res.find(r#""path":""#) {
        let after = &root_res[pos + 8..];
        if let Some(end_pos) = after.find('"') {
            let path = &after[..end_pos];
            if !path.is_empty() {
                return format!(r#", "cwd": "{}""#, json_escape(path));
            }
        }
    }
    String::new()
}

/// Git ステータスの更新処理 (Staged / Changes を完全分離)
fn refresh_git_status() {
    log("Refreshing Git status (VSCode style)...");
    let cwd_arg = get_cwd_arg();

    // 1. ブランチ名の取得 (git branch --show-current)
    let branch_cmd = format!(r#"{{"command": "git", "args": ["branch", "--show-current"]{}}}"#, cwd_arg);
    let branch_res = invoke("process.exec", &branch_cmd);
    let mut branch_name = "main".to_string();
    if let Some(pos) = branch_res.find(r#""stdout":""#) {
        let after = &branch_res[pos + 10..];
        if let Some(end_pos) = after.find('"') {
            let name = after[..end_pos].trim().replace("\\n", "").replace("\\r", "");
            if !name.is_empty() {
                branch_name = name;
            }
        }
    }

    // 2. Status Bar の更新 (ブランチ名表示)
    let status_bar_args = format!(
        r#"{{"id": "git_branch", "text": " {}", "align": "left", "command": "git.refresh"}}"#,
        json_escape(&branch_name)
    );
    invoke("ui.register_status_bar_item", &status_bar_args);

    // 3. git status --porcelain の実行
    let status_cmd = format!(r#"{{"command": "git", "args": ["status", "--porcelain"]{}}}"#, cwd_arg);
    let status_res = invoke("process.exec", &status_cmd);

    let mut staged_nodes = Vec::new();
    let mut changes_nodes = Vec::new();
    let mut git_stats_map = Vec::new();

    if let Some(pos) = status_res.find(r#""stdout":""#) {
        let after = &status_res[pos + 10..];
        if let Some(end_pos) = after.find('"') {
            let stdout_raw = &after[..end_pos];
            let stdout = stdout_raw.replace("\\n", "\n").replace("\\r", "\r");

            for line in stdout.lines() {
                if line.len() < 3 {
                    continue;
                }
                let index_status = line.chars().nth(0).unwrap_or(' ');
                let worktree_status = line.chars().nth(1).unwrap_or(' ');
                let file_path = line[3..].trim();

                // ファイル名と親ディレクトリの分割
                let (file_name, dir_path) = if let Some(last_slash) = file_path.rfind('/') {
                    (&file_path[last_slash + 1..], &file_path[..last_slash])
                } else if let Some(last_slash) = file_path.rfind('\\') {
                    (&file_path[last_slash + 1..], &file_path[..last_slash])
                } else {
                    (file_path, "")
                };

                // Staged の判定
                if index_status != ' ' && index_status != '?' {
                    let status_str = index_status.to_string();
                    let node = format!(
                        r#"{{"name": "{}", "dir": "{}", "path": "{}", "status": "{}", "staged": true, "icon": "file"}}"#,
                        json_escape(file_name),
                        json_escape(dir_path),
                        json_escape(file_path),
                        json_escape(&status_str)
                    );
                    staged_nodes.push(node);
                }

                // Changes (Unstaged) の判定
                if worktree_status != ' ' || index_status == '?' {
                    let status_str = if index_status == '?' && worktree_status == '?' {
                        "U".to_string()
                    } else {
                        worktree_status.to_string()
                    };
                    let node = format!(
                        r#"{{"name": "{}", "dir": "{}", "path": "{}", "status": "{}", "staged": false, "icon": "file"}}"#,
                        json_escape(file_name),
                        json_escape(dir_path),
                        json_escape(file_path),
                        json_escape(&status_str)
                    );
                    changes_nodes.push(node);

                    git_stats_map.push(format!(
                        r#""{}": "{}""#,
                        json_escape(file_path),
                        json_escape(&status_str)
                    ));
                } else if index_status != ' ' {
                    // Staged のみの場合もエクスプローラーには反映
                    git_stats_map.push(format!(
                        r#""{}": "{}""#,
                        json_escape(file_path),
                        json_escape(&index_status.to_string())
                    ));
                }
            }
        }
    }

    // 4. Source Control サイドバー UI AST の登録
    let total_changes = staged_nodes.len() + changes_nodes.len();
    let staged_array = staged_nodes.join(", ");
    let changes_array = changes_nodes.join(", ");

    let ui_ast = format!(
        r#"{{
            "type": "source_control",
            "branch": "{}",
            "total_count": {},
            "staged_nodes": [{}],
            "changes_nodes": [{}]
        }}"#,
        json_escape(&branch_name),
        total_changes,
        staged_array,
        changes_array
    );

    let sidebar_args = format!(
        r#"{{"id": "git_sidebar", "title": "SOURCE CONTROL", "ui": {}}}"#,
        ui_ast
    );
    invoke("ui.register_sidebar", &sidebar_args);

    // 5. settings ("git.status") の更新（エクスプローラーのバッジ連動）
    let git_stats_json = format!("{{{}}}", git_stats_map.join(", "));
    let settings_args = format!(r#"{{"key": "git.status", "value": {}}}"#, git_stats_json);
    invoke("settings.set", &settings_args);

    log("Git status refresh completed.");
}

/// プラグイン初期化
#[unsafe(no_mangle)]
pub extern "C" fn init() {
    log("Git Plugin initializing...");

    // Activity Bar に Source Control アイコンを登録
    invoke(
        "ui.register_activity_bar_item",
        r#"{"id": "git_sidebar", "icon": "source_control", "tooltip": "Source Control", "command": "git.open_sidebar"}"#
    );

    // コマンド登録
    invoke("command.register", r#"{"command": "git.refresh"}"#);
    invoke("command.register", r#"{"command": "git.commit"}"#);
    invoke("command.register", r#"{"command": "git.open_sidebar"}"#);
    invoke("command.register", r#"{"command": "git.stage"}"#);
    invoke("command.register", r#"{"command": "git.unstage"}"#);
    invoke("command.register", r#"{"command": "git.discard"}"#);

    // 初回ステータス取得
    refresh_git_status();

    log("Git Plugin initialized successfully.");
}

/// ホストからのイベント通知リスナー
#[unsafe(no_mangle)]
pub extern "C" fn on_event(ptr: i32, len: i32) {
    let slice = unsafe { std::slice::from_raw_parts(ptr as *const u8, len as usize) };
    let json_str = String::from_utf8_lossy(slice);
    let cwd_arg = get_cwd_arg();

    if json_str.contains("command_execute") {
        if json_str.contains("git.refresh") {
            refresh_git_status();
        } else if json_str.contains("git.open_sidebar") {
            invoke("panel.open", r#"{"id": "git_sidebar"}"#);
            refresh_git_status();
        } else if json_str.contains("git.stage:") {
            // "git.stage:path/to/file"
            if let Some(pos) = json_str.find("git.stage:") {
                let after = &json_str[pos + 10..];
                let file = after.trim_matches(|c| c == '"' || c == '}' || c == ' ' || c == '\n');
                let add_cmd = format!(r#"{{"command": "git", "args": ["add", "{}"]{}}}"#, json_escape(file), cwd_arg);
                invoke("process.exec", &add_cmd);
                refresh_git_status();
            }
        } else if json_str.contains("git.unstage:") {
            // "git.unstage:path/to/file"
            if let Some(pos) = json_str.find("git.unstage:") {
                let after = &json_str[pos + 12..];
                let file = after.trim_matches(|c| c == '"' || c == '}' || c == ' ' || c == '\n');
                let restore_cmd = format!(r#"{{"command": "git", "args": ["restore", "--staged", "{}"]{}}}"#, json_escape(file), cwd_arg);
                invoke("process.exec", &restore_cmd);
                refresh_git_status();
            }
        } else if json_str.contains("git.discard:") {
            // "git.discard:path/to/file"
            if let Some(pos) = json_str.find("git.discard:") {
                let after = &json_str[pos + 12..];
                let file = after.trim_matches(|c| c == '"' || c == '}' || c == ' ' || c == '\n');
                let restore_cmd = format!(r#"{{"command": "git", "args": ["restore", "{}"]{}}}"#, json_escape(file), cwd_arg);
                invoke("process.exec", &restore_cmd);
                refresh_git_status();
            }
        } else if json_str.contains("git.stage_all") {
            let add_all_cmd = format!(r#"{{"command": "git", "args": ["add", "-A"]{}}}"#, cwd_arg);
            invoke("process.exec", &add_all_cmd);
            refresh_git_status();
        } else if json_str.contains("git.unstage_all") {
            let unstage_all_cmd = format!(r#"{{"command": "git", "args": ["restore", "--staged", "."]{}}}"#, cwd_arg);
            invoke("process.exec", &unstage_all_cmd);
            refresh_git_status();
        } else if json_str.contains("git.discard_all") {
            let discard_all_cmd = format!(r#"{{"command": "git", "args": ["restore", "."]{}}}"#, cwd_arg);
            invoke("process.exec", &discard_all_cmd);
            refresh_git_status();
        } else if json_str.contains("git.commit") {
            log("Executing git commit...");
            let commit_cmd = format!(r#"{{"command": "git", "args": ["commit", "-m", "Commit from Nucleus"]{}}}"#, cwd_arg);
            invoke("process.exec", &commit_cmd);
            refresh_git_status();
        }
    } else if json_str.contains("file_opened") || json_str.contains("fs_write_complete") {
        refresh_git_status();
    }
}
