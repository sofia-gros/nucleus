//! プログラムを動かす（git など）。終わるまで待って、出力をまとめて返す。

use std::{
    io::Write as _,
    process::{Command, Stdio},
};

use remote::{ProcessOutput, ProcessSpec};

pub fn run(spec: ProcessSpec) -> std::io::Result<ProcessOutput> {
    let mut cmd = Command::new(&spec.program);
    cmd.args(&spec.args)
        .envs(spec.env.iter().map(|(k, v)| (k, v)))
        .stdin(if spec.stdin.is_some() {
            Stdio::piped()
        } else {
            Stdio::null()
        })
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    if let Some(cwd) = &spec.cwd {
        cmd.current_dir(cwd);
    }
    // Windows では、黒いコンソール画面を出さない。
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt as _;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }
    let mut child = cmd.spawn()?;
    if let Some(input) = spec.stdin
        && let Some(mut stdin) = child.stdin.take()
    {
        // 大きな入力でも詰まらないように、書くのは別のスレッドで。
        std::thread::spawn(move || {
            let _ = stdin.write_all(&input);
        });
    }
    let out = child.wait_with_output()?;
    Ok(ProcessOutput {
        code: out.status.code(),
        stdout: out.stdout,
        stderr: out.stderr,
    })
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;

    #[test]
    fn runs_with_input_and_reports_failure() {
        let out = run(ProcessSpec {
            program: "sh".into(),
            args: vec!["-c".into(), "cat; echo err >&2; exit 3".into()],
            stdin: Some(b"hello".to_vec()),
            ..Default::default()
        })
        .unwrap();
        assert_eq!(out.stdout, b"hello");
        assert_eq!(out.stderr, b"err\n");
        assert_eq!(out.code, Some(3));
        assert!(!out.success());
        assert!(
            run(ProcessSpec {
                program: "no-such-program-xyz".into(),
                ..Default::default()
            })
            .is_err()
        );
    }
}
