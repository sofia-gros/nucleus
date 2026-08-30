/// ターミナル PTY 出力ストリーム解析 & ANSI 除去テスト

use nucleus::terminal::TerminalSession;

#[test]
fn test_strip_ansi_csi_and_osc_sequences() {
    let input = "\x1b[31mError:\x1b[0m File not found\x1b[2K\r\n\x1b]0;Title\x07PS C:\\Project> ";
    let cleaned = TerminalSession::strip_ansi_codes(input);
    assert_eq!(cleaned, "Error: File not found\r\nPS C:\\Project> ");
}

#[test]
fn test_multiline_powershell_banner_split() {
    let raw_banner = "PowerShell 7.4.0\r\nCopyright (C) Microsoft Corporation.\r\n\r\nPS C:\\Project\\nucleus> ";
    let cleaned = TerminalSession::strip_ansi_codes(raw_banner);
    
    let mut lines = Vec::new();
    let mut current_line = String::new();
    let mut chars = cleaned.chars().peekable();

    while let Some(ch) = chars.next() {
        if ch == '\r' {
            if chars.peek() == Some(&'\n') {
                chars.next();
                lines.push(current_line.clone());
                current_line.clear();
            } else {
                current_line.clear();
            }
        } else if ch == '\n' {
            lines.push(current_line.clone());
            current_line.clear();
        } else {
            current_line.push(ch);
        }
    }
    if !current_line.is_empty() {
        lines.push(current_line);
    }

    assert_eq!(lines.len(), 4);
    assert_eq!(lines[0], "PowerShell 7.4.0");
    assert_eq!(lines[1], "Copyright (C) Microsoft Corporation.");
    assert_eq!(lines[2], "");
    assert_eq!(lines[3], "PS C:\\Project\\nucleus> ");
}
