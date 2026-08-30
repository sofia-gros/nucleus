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
            let mut buf = [0u8; 4096];
            let mut current_line = String::new();

            while let Ok(n) = reader.read(&mut buf) {
                if n == 0 {
                    break;
                }
                let raw_text = String::from_utf8_lossy(&buf[..n]);
                let clean_text = Self::strip_ansi_codes(&raw_text);
                let mut chars = clean_text.chars().peekable();

                while let Some(ch) = chars.next() {
                    if ch == '\r' {
                        if chars.peek() == Some(&'\n') {
                            chars.next(); // '\n' を消費して CRLF 確定
                            if let Ok(mut lines) = output_lines_clone.write() {
                                lines.push(current_line.clone());
                                if lines.len() > 2000 {
                                    lines.remove(0);
                                }
                            }
                            current_line.clear();
                        } else {
                            // 単独の CR（行頭復帰）
                            current_line.clear();
                        }
                    } else if ch == '\n' {
                        // 単独の LF
                        if let Ok(mut lines) = output_lines_clone.write() {
                            lines.push(current_line.clone());
                            if lines.len() > 2000 {
                                lines.remove(0);
                            }
                        }
                        current_line.clear();
                    } else if ch != '\0' {
                        current_line.push(ch);
                    }
                }

                // 未改行のプロンプト行（例: "PS C:\Project\nucleus> "）を即座にUIへ反映
                if !current_line.is_empty() {
                    if let Ok(mut lines) = output_lines_clone.write() {
                        if lines.is_empty() {
                            lines.push(current_line.clone());
                        } else {
                            let last = lines.last_mut().unwrap();
                            *last = current_line.clone();
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

    /// ANSI エスケープコード・OSC・VT100 制御シーケンスの完全除去
    pub fn strip_ansi_codes(input: &str) -> String {
        let mut result = String::with_capacity(input.len());
        let mut chars = input.chars().peekable();

        while let Some(ch) = chars.next() {
            if ch == '\x1B' {
                // ESC シーケンスの開始
                if let Some(&next_ch) = chars.peek() {
                    if next_ch == '[' {
                        chars.next(); // '[' を消費
                        // CSI シーケンス: 終端文字 (0x40..=0x7E) が来るまでスキップ
                        while let Some(&c) = chars.peek() {
                            chars.next();
                            if c >= '@' && c <= '~' {
                                break;
                            }
                        }
                    } else if next_ch == ']' {
                        chars.next(); // ']' を消費
                        // OSC シーケンス: BEL (\x07) または ST (\x1b\) が来るまでスキップ
                        while let Some(c) = chars.next() {
                            if c == '\x07' || c == '\x1B' {
                                break;
                            }
                        }
                    } else {
                        chars.next();
                    }
                }
            } else {
                result.push(ch);
            }
        }
        result
    }
}
