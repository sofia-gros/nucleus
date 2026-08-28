/// PTY (Pseudo-Terminal) セッション管理および端末エミュレーションモジュール

use anyhow::{Context, Result};
use portable_pty::{native_pty_system, CommandBuilder, MasterPty, PtySize};
use std::io::{Read, Write};
use std::path::PathBuf;
use std::sync::{Arc, RwLock};
use std::thread;

/// 1つのアクティブなターミナル PTY セッション
pub struct TerminalSession {
    pub id: String,
    pub title: String,
    master: Box<dyn MasterPty + Send>,
    writer: Arc<RwLock<Box<dyn Write + Send>>>,
    pub output_lines: Arc<RwLock<Vec<String>>>,
    pub current_input: Arc<RwLock<String>>,
}

impl TerminalSession {
    /// 新規 PTY ターミナルセッションの作成とシェルの起動
    pub fn new(id: String, title: String, cwd: Option<PathBuf>) -> Result<Self> {
        let pty_system = native_pty_system();
        let pair = pty_system
            .openpty(PtySize {
                rows: 24,
                cols: 80,
                pixel_width: 0,
                pixel_height: 0,
            })
            .context("Failed to open PTY pair")?;

        let shell = Self::detect_default_shell();
        let mut cmd = CommandBuilder::new(shell);
        if let Some(dir) = cwd {
            cmd.cwd(dir);
        }

        let _child = pair.slave.spawn_command(cmd).context("Failed to spawn shell process in PTY")?;
        drop(pair.slave); // Master側のみ保持

        let mut reader = pair.master.try_clone_reader().context("Failed to clone PTY reader")?;
        let writer = Arc::new(RwLock::new(pair.master.take_writer().context("Failed to take PTY writer")?));

        let output_lines = Arc::new(RwLock::new(Vec::new()));
        let output_lines_clone = output_lines.clone();

        // バックグラウンドスレッドで PTY 出力を継続読み取り
        thread::spawn(move || {
            let mut buf = [0u8; 1024];
            let mut line_buffer = String::new();

            while let Ok(n) = reader.read(&mut buf) {
                if n == 0 {
                    break;
                }
                let s = String::from_utf8_lossy(&buf[..n]);
                for ch in s.chars() {
                    if ch == '\n' {
                        let clean_line = Self::strip_ansi_codes(&line_buffer);
                        if let Ok(mut lines) = output_lines_clone.write() {
                            lines.push(clean_line);
                            // 最大保持行数制限
                            if lines.len() > 1000 {
                                lines.remove(0);
                            }
                        }
                        line_buffer.clear();
                    } else if ch == '\r' {
                        // CR は無視
                    } else {
                        line_buffer.push(ch);
                    }
                }
                if !line_buffer.is_empty() {
                    let clean_line = Self::strip_ansi_codes(&line_buffer);
                    if let Ok(mut lines) = output_lines_clone.write() {
                        if lines.is_empty() {
                            lines.push(clean_line);
                        } else {
                            let last = lines.last_mut().unwrap();
                            *last = clean_line;
                        }
                    }
                }
            }
        });

        Ok(Self {
            id,
            title,
            master: pair.master,
            writer,
            output_lines,
            current_input: Arc::new(RwLock::new(String::new())),
        })
    }

    /// PTY へのテキスト書き込み
    pub fn write_input(&self, input: &str) -> Result<()> {
        if let Ok(mut w) = self.writer.write() {
            w.write_all(input.as_bytes())?;
            w.flush()?;
        }
        Ok(())
    }

    /// 端末サイズの変更
    pub fn resize(&self, rows: u16, cols: u16) -> Result<()> {
        self.master.resize(PtySize {
            rows,
            cols,
            pixel_width: 0,
            pixel_height: 0,
        })?;
        Ok(())
    }

    /// OS に応じたデフォルトシェルの検出
    fn detect_default_shell() -> String {
        #[cfg(target_os = "windows")]
        {
            if std::path::Path::new("C:\\Program Files\\PowerShell\\7\\pwsh.exe").exists() {
                "C:\\Program Files\\PowerShell\\7\\pwsh.exe".to_string()
            } else {
                "powershell.exe".to_string()
            }
        }
        #[cfg(not(target_os = "windows"))]
        {
            std::env::var("SHELL").unwrap_or_else(|_| "/bin/bash".to_string())
        }
    }

    /// ANSI エスケープコードの簡易除去
    fn strip_ansi_codes(input: &str) -> String {
        let mut result = String::with_capacity(input.len());
        let mut in_escape = false;

        for ch in input.chars() {
            if ch == '\x1B' {
                in_escape = true;
            } else if in_escape {
                if (ch >= 'A' && ch <= 'Z') || (ch >= 'a' && ch <= 'z') || ch == '~' {
                    in_escape = false;
                }
            } else {
                result.push(ch);
            }
        }
        result
    }
}
