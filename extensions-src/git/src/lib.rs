//! 公式の Git Extension。`git` コマンドを呼んで、変更の一覧・ステージ・コミットを右ドックに出す。
//! `build.sh` で WASM にして extensions/nucleus.git/ に置く。

use std::cell::RefCell;

use nucleus_extension_api::{
    Extension, WorkspaceEvent, export_extension, host,
    ui::{
        ButtonVariant, El, Size, Tone, UiEvent, button, divider, h_stack, input, list_item, scroll, section, text,
        v_stack,
    },
};

mod status;

use status::{Entry, Status};

#[derive(Default)]
struct State {
    status: Option<Result<Status, String>>,
    message: String,
    /// コミット後に入力欄を空にするために増やす。
    message_revision: u32,
    busy: bool,
}

thread_local! {
    static STATE: RefCell<State> = RefCell::new(State::default());
}

fn with<R>(f: impl FnOnce(&mut State) -> R) -> R {
    STATE.with(|s| f(&mut s.borrow_mut()))
}

fn git(args: &[&str]) -> Result<String, String> {
    let args: Vec<String> = args.iter().map(|s| s.to_string()).collect();
    let out = host::run_process("git", &args)?;
    if out.status == 0 {
        Ok(out.stdout)
    } else {
        let msg = if out.stderr.trim().is_empty() {
            out.stdout
        } else {
            out.stderr
        };
        Err(msg.trim().to_string())
    }
}

fn refresh() {
    let status = if host::workspace_root().is_none() {
        Err(String::new())
    } else {
        git(&["status", "--porcelain=v1", "-b", "-z", "--untracked-files=all"]).map(|s| status::parse(&s))
    };
    with(|s| s.status = Some(status));
}

fn notify_err(r: Result<String, String>) {
    if let Err(e) = r {
        host::notify(host::NotificationLevel::Error, &e);
    }
}

fn commit() -> Result<(), String> {
    let (message, staged) = with(|s| {
        let staged = matches!(&s.status, Some(Ok(st)) if !st.staged.is_empty());
        (s.message.trim().to_string(), staged)
    });
    if !staged {
        host::notify(
            host::NotificationLevel::Warning,
            "There are no staged changes to commit.",
        );
        return Ok(());
    }
    if message.is_empty() {
        host::notify(host::NotificationLevel::Warning, "Enter a commit message.");
        return Ok(());
    }
    with(|s| s.busy = true);
    let r = git(&["commit", "-q", "-m", &message]);
    with(|s| s.busy = false);
    match r {
        Ok(_) => {
            with(|s| {
                s.message.clear();
                s.message_revision += 1;
            });
            refresh();
            Ok(())
        }
        Err(e) => Err(e),
    }
}

/// 差分を開く。未ステージの変更はインデックスと、ステージ済みの変更は HEAD と比べる。
fn show_diff(path: &str, staged: bool, code: &str) -> Result<(), String> {
    if matches!(code, "U" | "A") && !staged {
        return host::open_file(path, None);
    }
    let (spec, label) = if staged {
        (format!("HEAD:{path}"), "HEAD")
    } else {
        // ステージ済みの変更がなければ、インデックスは HEAD と同じ。
        let staged_too = with(|s| matches!(&s.status, Some(Ok(st)) if st.staged.iter().any(|e| e.path == path)));
        (format!(":{path}"), if staged_too { "Index" } else { "HEAD" })
    };
    let base = git(&["show", &spec]).unwrap_or_default();
    host::show_diff(path, &base, label)
}

/// push / fetch。時間がかかることがあるので、終わったら通知する。
fn remote(args: &[&str], done: &str) {
    with(|s| s.busy = true);
    let r = git(args);
    with(|s| s.busy = false);
    match r {
        Ok(_) => host::notify(host::NotificationLevel::Info, done),
        Err(e) => host::notify(host::NotificationLevel::Error, &e),
    }
    refresh();
}

struct Git;

impl Extension for Git {
    fn activate() -> Result<(), String> {
        refresh();
        Ok(())
    }

    fn run_command(id: String) -> Result<(), String> {
        match id.as_str() {
            "git.refresh" => refresh(),
            "git.stage-all" => {
                notify_err(git(&["add", "-A"]));
                refresh();
            }
            "git.commit" => commit()?,
            other => return Err(format!("unknown command `{other}`")),
        }
        Ok(())
    }

    fn on_event(_event: WorkspaceEvent) {
        refresh();
    }

    fn handle_ui_event(_view: String, key: String, event: UiEvent) -> Result<(), String> {
        match (key.as_str(), event) {
            ("message", UiEvent::Change(text)) => with(|s| s.message = text),
            ("stage-hunk", UiEvent::Submit(patch)) => {
                let out = host::run_process_input("git", &["apply".into(), "--cached".into(), "-".into()], &patch)?;
                if out.status != 0 {
                    return Err(out.stderr.trim().to_string());
                }
                refresh();
                // 差分の元（インデックス）を新しくする。
                if let Some(path) = patch
                    .lines()
                    .next()
                    .and_then(|l| l.strip_prefix("diff --git a/"))
                    .and_then(|l| l.split(" b/").next())
                {
                    show_diff(path, false, "M")?;
                }
            }
            ("message", UiEvent::Submit(text)) => {
                with(|s| s.message = text);
                commit()?;
            }
            ("commit", UiEvent::Click) => commit()?,
            ("refresh", UiEvent::Click) => refresh(),
            ("push", UiEvent::Click) => {
                remote(&["push", "-q"], "Pushed.");
            }
            ("fetch", UiEvent::Click) => {
                remote(&["fetch", "-q"], "Fetched.");
            }
            ("init", UiEvent::Click) => {
                notify_err(git(&["init", "-q"]));
                refresh();
            }
            ("stage-all", UiEvent::Click) => {
                notify_err(git(&["add", "-A"]));
                refresh();
            }
            ("unstage-all", UiEvent::Click) => {
                notify_err(git(&["reset", "-q"]));
                refresh();
            }
            (k, UiEvent::Click) => {
                if let Some(path) = k.strip_prefix("stage:") {
                    notify_err(git(&["add", "-A", "--", path]));
                    refresh();
                } else if let Some(path) = k.strip_prefix("unstage:") {
                    // HEAD がない（最初のコミット前）ときは reset が使えないので rm --cached にする。
                    if git(&["reset", "-q", "--", path]).is_err() {
                        notify_err(git(&["rm", "-q", "--cached", "--", path]));
                    }
                    refresh();
                } else if let Some(rest) = k.strip_prefix("diff:") {
                    // diff:<s|u>:<印>:<path>
                    let mut parts = rest.splitn(3, ':');
                    let (side, code, path) = (parts.next(), parts.next(), parts.next());
                    if let (Some(side), Some(code), Some(path)) = (side, code, path) {
                        show_diff(path, side == "s", code)?;
                    }
                } else if let Some(path) = k.strip_prefix("open:") {
                    host::open_file(path, None)?;
                }
            }
            _ => {}
        }
        Ok(())
    }

    fn render(_view: String) -> El {
        STATE.with(|s| view(&s.borrow()))
    }
}

fn view(s: &State) -> El {
    let status = match &s.status {
        None => return v_stack().child(nucleus_extension_api::ui::skeleton()),
        Some(Err(e)) if e.is_empty() => {
            return empty("Open a folder to use source control.", None);
        }
        Some(Err(e)) if e.contains("not a git repository") => {
            return empty(
                "This folder is not a git repository.",
                Some(
                    button("init")
                        .label("Initialize Repository")
                        .variant(ButtonVariant::Primary),
                ),
            );
        }
        Some(Err(e)) => return empty(e, Some(button("refresh").label("Retry").icon("refresh-cw"))),
        Some(Ok(st)) => st,
    };

    let mut lists = v_stack().padding(Size::Xs);
    lists = lists.child(
        section("changes", "Unstaged Changes")
            .count(status.unstaged.len() as u32)
            .action(
                button("stage-all")
                    .label("Stage All")
                    .variant(ButtonVariant::Ghost)
                    .disabled(status.unstaged.is_empty()),
            )
            .children(status.unstaged.iter().map(|e| row(e, false))),
    );
    lists = lists.child(
        section("staged", "Staged Changes")
            .count(status.staged.len() as u32)
            .action(
                button("unstage-all")
                    .label("Unstage All")
                    .variant(ButtonVariant::Ghost)
                    .disabled(status.staged.is_empty()),
            )
            .children(status.staged.iter().map(|e| row(e, true))),
    );
    if status.staged.is_empty() && status.unstaged.is_empty() {
        lists = lists.child(
            v_stack().padding(Size::Sm).child(
                text("No changes. The working tree is clean.")
                    .tone(Tone::Muted)
                    .size(Size::Xs),
            ),
        );
    }

    // 1行目の長さ（72 文字が目安）。
    let first_line = s.message.lines().next().unwrap_or("").chars().count();
    let can_commit = !status.staged.is_empty() && !s.message.trim().is_empty() && !s.busy;
    let commit_box = v_stack()
        .gap(Size::Sm)
        .padding(Size::Sm)
        .child(
            input("message")
                .placeholder("Commit message (Ctrl+Enter to commit)")
                .revision(s.message_revision)
                .multiline(),
        )
        .child(
            h_stack()
                .spread()
                .child(
                    text(format!("{first_line} / 72"))
                        .tone(if first_line > 72 { Tone::Warning } else { Tone::Muted })
                        .size(Size::Xs),
                )
                .child(
                    button("commit")
                        .label(format!("Commit to {}", status.branch))
                        .variant(ButtonVariant::Primary)
                        .disabled(!can_commit),
                ),
        );

    let footer = h_stack()
        .spread()
        .padding(Size::Xs)
        .child(
            h_stack()
                .gap(Size::Xs)
                .child(nucleus_extension_api::ui::icon("git-branch").tone(Tone::Muted))
                .child(text(status.branch.clone()).tone(Tone::Muted).size(Size::Xs)),
        )
        .child(
            h_stack()
                .gap(Size::Xs)
                .child(
                    button("push")
                        .label(if status.ahead > 0 {
                            format!("Push ↑{}", status.ahead)
                        } else {
                            "Push".into()
                        })
                        .variant(ButtonVariant::Ghost)
                        .disabled(s.busy),
                )
                .child(
                    button("fetch")
                        .label(if status.behind > 0 {
                            format!("Fetch ↓{}", status.behind)
                        } else {
                            "Fetch".into()
                        })
                        .variant(ButtonVariant::Ghost)
                        .disabled(s.busy),
                )
                .child(
                    button("refresh")
                        .icon("refresh-cw")
                        .variant(ButtonVariant::Ghost)
                        .tooltip("Refresh"),
                ),
        );

    v_stack()
        .grow()
        .child(scroll().child(lists))
        .child(divider())
        .child(commit_box)
        .child(divider())
        .child(footer)
}

fn row(e: &Entry, staged: bool) -> El {
    let (name, dir) = match e.path.rsplit_once('/') {
        Some((d, n)) => (n.to_string(), d.to_string()),
        None => (e.path.clone(), String::new()),
    };
    let tone = match e.code {
        'M' => Tone::Warning,
        'A' | 'U' => Tone::Success,
        'D' => Tone::Danger,
        'R' | 'C' => Tone::Info,
        _ => Tone::Muted,
    };
    let toggle = if staged {
        button(format!("unstage:{}", e.path))
            .icon("minus")
            .variant(ButtonVariant::Ghost)
            .tooltip("Unstage")
    } else {
        button(format!("stage:{}", e.path))
            .icon("plus")
            .variant(ButtonVariant::Ghost)
            .tooltip("Stage")
    };
    let side = if staged { "s" } else { "u" };
    let mut item = list_item(format!("diff:{side}:{}:{}", e.code, e.path), name)
        .icon("file")
        .tone(Tone::Muted)
        .badge(e.code.to_string(), tone)
        .action(
            button(format!("open:{}", e.path))
                .icon("file-code")
                .variant(ButtonVariant::Ghost)
                .tooltip("Open File"),
        )
        .action(toggle);
    if !dir.is_empty() {
        item = item.detail(dir);
    }
    item
}

fn empty(message: &str, action: Option<El>) -> El {
    let mut v = v_stack()
        .gap(Size::Sm)
        .padding(Size::Md)
        .child(text(message.to_string()).tone(Tone::Muted).size(Size::Xs));
    if let Some(a) = action {
        v = v.child(h_stack().child(a));
    }
    v
}

export_extension!(Git);
