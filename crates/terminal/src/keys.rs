//! キー入力を端末に送るバイト列に変える（xterm 互換）。

/// 修飾キー。
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Mods {
    pub ctrl: bool,
    pub alt: bool,
    pub shift: bool,
}

impl Mods {
    /// xterm の修飾パラメータ（1 + shift + 2*alt + 4*ctrl）。修飾なしは None。
    fn param(self) -> Option<u8> {
        let m = self.shift as u8 + 2 * self.alt as u8 + 4 * self.ctrl as u8;
        (m > 0).then_some(m + 1)
    }
}

/// `key` は gpui の Keystroke::key（"a", "enter", "up", "f5" など）、`key_char` は入力された文字。
/// `app_cursor` は DECCKM（アプリケーションカーソルキー）モード。
pub fn to_bytes(key: &str, key_char: Option<&str>, mods: Mods, app_cursor: bool) -> Option<Vec<u8>> {
    let csi = |s: &str| format!("\x1b[{s}").into_bytes();
    let cursor = |c: char| match mods.param() {
        Some(m) => format!("\x1b[1;{m}{c}").into_bytes(),
        None if app_cursor => format!("\x1bO{c}").into_bytes(),
        None => format!("\x1b[{c}").into_bytes(),
    };
    let tilde = |n: u8| match mods.param() {
        Some(m) => csi(&format!("{n};{m}~")),
        None => csi(&format!("{n}~")),
    };
    let ss3 = |c: char| match mods.param() {
        Some(m) => csi(&format!("1;{m}{c}")),
        None => format!("\x1bO{c}").into_bytes(),
    };
    let bytes = match key {
        "enter" => {
            if mods.alt {
                b"\x1b\r".to_vec()
            } else {
                b"\r".to_vec()
            }
        }
        "tab" if mods.shift => csi("Z"),
        "tab" => b"\t".to_vec(),
        "backspace" if mods.ctrl => b"\x08".to_vec(),
        "backspace" if mods.alt => b"\x1b\x7f".to_vec(),
        "backspace" => b"\x7f".to_vec(),
        "escape" => b"\x1b".to_vec(),
        "up" => cursor('A'),
        "down" => cursor('B'),
        "right" => cursor('C'),
        "left" => cursor('D'),
        "home" => cursor('H'),
        "end" => cursor('F'),
        "pageup" => tilde(5),
        "pagedown" => tilde(6),
        "insert" => tilde(2),
        "delete" => tilde(3),
        "f1" => ss3('P'),
        "f2" => ss3('Q'),
        "f3" => ss3('R'),
        "f4" => ss3('S'),
        "f5" => tilde(15),
        "f6" => tilde(17),
        "f7" => tilde(18),
        "f8" => tilde(19),
        "f9" => tilde(20),
        "f10" => tilde(21),
        "f11" => tilde(23),
        "f12" => tilde(24),
        "space" if mods.ctrl => vec![0],
        _ if mods.ctrl && !mods.shift && key.chars().count() == 1 => {
            let c = key.chars().next()?.to_ascii_lowercase();
            let b = match c {
                'a'..='z' => c as u8 & 0x1f,
                '@' | '2' => 0,
                '[' | '3' => 0x1b,
                '\\' | '4' => 0x1c,
                ']' | '5' => 0x1d,
                '^' | '6' => 0x1e,
                '_' | '-' | '7' => 0x1f,
                '?' | '8' => 0x7f,
                _ => return None,
            };
            if mods.alt { vec![0x1b, b] } else { vec![b] }
        }
        _ => {
            let text = key_char.or(if key == "space" { Some(" ") } else { None })?;
            if text.is_empty() {
                return None;
            }
            let mut v = Vec::new();
            if mods.alt {
                v.push(0x1b);
            }
            v.extend_from_slice(text.as_bytes());
            v
        }
    };
    Some(bytes)
}

/// 貼り付け。ブラケットペーストモードなら印で囲み、改行は CR にそろえる。
pub fn paste(text: &str, bracketed: bool) -> Vec<u8> {
    let body = text.replace("\r\n", "\r").replace('\n', "\r");
    if bracketed {
        // 貼り付けの終わりの印を中に含めない（貼り付けからのコマンド注入を防ぐ）。
        let body = body.replace("\x1b[201~", "");
        format!("\x1b[200~{body}\x1b[201~").into_bytes()
    } else {
        body.into_bytes()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const NONE: Mods = Mods {
        ctrl: false,
        alt: false,
        shift: false,
    };
    const CTRL: Mods = Mods {
        ctrl: true,
        alt: false,
        shift: false,
    };

    fn s(key: &str, ch: Option<&str>, m: Mods) -> Vec<u8> {
        to_bytes(key, ch, m, false).unwrap()
    }

    #[test]
    fn plain_text_and_controls() {
        assert_eq!(s("a", Some("a"), NONE), b"a");
        assert_eq!(s("a", Some("A"), Mods { shift: true, ..NONE }), b"A");
        assert_eq!(s("enter", None, NONE), b"\r");
        assert_eq!(s("backspace", None, NONE), b"\x7f");
        assert_eq!(s("c", None, CTRL), [3]);
        assert_eq!(s("[", None, CTRL), [0x1b]);
        assert_eq!(s("space", None, CTRL), [0]);
        assert_eq!(s("b", Some("b"), Mods { alt: true, ..NONE }), b"\x1bb");
        assert_eq!(s("space", None, NONE), b" ");
        assert_eq!(s("あ", Some("あ"), NONE), "あ".as_bytes());
    }

    #[test]
    fn cursor_keys_follow_mode_and_modifiers() {
        assert_eq!(s("up", None, NONE), b"\x1b[A");
        assert_eq!(to_bytes("up", None, NONE, true).unwrap(), b"\x1bOA");
        assert_eq!(s("right", None, CTRL), b"\x1b[1;5C");
        assert_eq!(s("tab", None, Mods { shift: true, ..NONE }), b"\x1b[Z");
        assert_eq!(s("delete", None, NONE), b"\x1b[3~");
        assert_eq!(s("f1", None, NONE), b"\x1bOP");
        assert_eq!(s("f5", None, NONE), b"\x1b[15~");
        assert_eq!(s("pageup", None, Mods { shift: true, ..NONE }), b"\x1b[5;2~");
    }

    #[test]
    fn unknown_keys_send_nothing() {
        assert!(to_bytes("shift", None, NONE, false).is_none());
        assert!(to_bytes("f30", None, NONE, false).is_none());
    }

    #[test]
    fn paste_brackets_and_strips_injected_end_marker() {
        assert_eq!(paste("a\nb", false), b"a\rb");
        assert_eq!(paste("x\x1b[201~rm -rf", true), b"\x1b[200~xrm -rf\x1b[201~");
    }
}
