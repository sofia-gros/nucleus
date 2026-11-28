//! 1つの塊だけのパッチが `git apply --cached` で当たることを、本物の git で確かめる。

use std::{io::Write as _, process::Command};

use editor::diff;

fn git(dir: &std::path::Path, args: &[&str], input: Option<&str>) -> String {
    let mut child = Command::new("git")
        .args(args)
        .current_dir(dir)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .unwrap();
    if let Some(i) = input {
        child.stdin.take().unwrap().write_all(i.as_bytes()).unwrap();
    }
    let out = child.wait_with_output().unwrap();
    assert!(
        out.status.success(),
        "git {args:?}: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8(out.stdout).unwrap()
}

#[test]
fn staging_one_hunk() {
    if Command::new("git").arg("--version").output().is_err() {
        return;
    }
    let d = tempfile::tempdir().unwrap();
    let p = d.path();
    let old: String = (1..=30).map(|i| format!("line {i}\n")).collect();
    std::fs::write(p.join("f.txt"), &old).unwrap();
    git(p, &["init", "-q"], None);
    git(p, &["add", "."], None);
    git(
        p,
        &["-c", "user.name=t", "-c", "user.email=t@t", "commit", "-qm", "init"],
        None,
    );
    let new = old
        .replace("line 2\n", "line two\n")
        .replace("line 28\n", "line 28\nline 28.5\n");
    std::fs::write(p.join("f.txt"), &new).unwrap();

    let hunks = diff::compute(&old, &new, 3);
    assert_eq!(hunks.len(), 2);
    let patch = diff::hunk_patch("f.txt", &hunks[1], true, true);
    git(p, &["apply", "--cached", "-"], Some(&patch));
    let staged = git(p, &["diff", "--cached"], None);
    assert!(staged.contains("+line 28.5"), "{staged}");
    assert!(!staged.contains("line two"), "{staged}");
}
