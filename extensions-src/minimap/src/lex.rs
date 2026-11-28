//! 縮図用のおおまかな字句の色分け（言語を問わない）。正確さより速さを優先する。

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Class {
    Text,
    Keyword,
    Function,
    Type,
    String,
    Number,
    Comment,
}

impl Class {
    pub fn token(self) -> &'static str {
        match self {
            Class::Text => "highlight.editor.foreground",
            Class::Keyword => "highlight.syntax.keyword",
            Class::Function => "highlight.syntax.function",
            Class::Type => "highlight.syntax.type",
            Class::String => "highlight.syntax.string",
            Class::Number => "highlight.syntax.number",
            Class::Comment => "highlight.syntax.comment",
        }
    }
}

const KEYWORDS: &[&str] = &[
    "fn",
    "let",
    "mut",
    "pub",
    "use",
    "mod",
    "impl",
    "struct",
    "enum",
    "trait",
    "match",
    "if",
    "else",
    "for",
    "while",
    "loop",
    "return",
    "const",
    "static",
    "async",
    "await",
    "move",
    "where",
    "self",
    "Self",
    "crate",
    "super",
    "type",
    "function",
    "var",
    "class",
    "def",
    "import",
    "from",
    "export",
    "new",
    "this",
    "interface",
    "package",
    "func",
    "go",
    "true",
    "false",
    "null",
    "None",
    "nil",
    "in",
    "of",
    "break",
    "continue",
    "try",
    "catch",
    "with",
    "as",
];

/// 行ごとの（開始列, 長さ, 色）。`width` 列より右は切る。タブは 4 列。
pub fn analyze(text: &str, width: usize) -> Vec<Vec<(u16, u16, Class)>> {
    text.lines().map(|l| line(l, width)).collect()
}

fn line(l: &str, width: usize) -> Vec<(u16, u16, Class)> {
    let chars: Vec<char> = l.chars().collect();
    let mut out = Vec::new();
    let mut col = 0usize;
    let mut i = 0;
    let push = |out: &mut Vec<(u16, u16, Class)>, start: usize, len: usize, class: Class| {
        if start < width && len > 0 {
            let len = len.min(width - start);
            // 隣が同じ色ならつなげる（四角形の数を減らす）。
            if let Some(last) = out.last_mut()
                && last.2 == class
                && last.0 as usize + last.1 as usize + 1 >= start
            {
                last.1 = (start + len - last.0 as usize) as u16;
                return;
            }
            out.push((start as u16, len as u16, class));
        }
    };
    while i < chars.len() && col < width {
        let c = chars[i];
        if c == '\t' {
            col += 4;
            i += 1;
            continue;
        }
        if c.is_whitespace() {
            col += 1;
            i += 1;
            continue;
        }
        // コメント（行の残りすべて）
        if (c == '/' && chars.get(i + 1) == Some(&'/')) || c == '#' || (c == '-' && chars.get(i + 1) == Some(&'-')) {
            push(&mut out, col, chars.len() - i, Class::Comment);
            break;
        }
        if c == '"' || c == '\'' || c == '`' {
            let start = i;
            i += 1;
            while i < chars.len() && chars[i] != c {
                if chars[i] == '\\' {
                    i += 1;
                }
                i += 1;
            }
            i = (i + 1).min(chars.len());
            push(&mut out, col, i - start, Class::String);
            col += i - start;
            continue;
        }
        if c.is_alphanumeric() || c == '_' {
            let start = i;
            while i < chars.len() && (chars[i].is_alphanumeric() || chars[i] == '_') {
                i += 1;
            }
            let word: String = chars[start..i].iter().collect();
            let class = if c.is_ascii_digit() {
                Class::Number
            } else if KEYWORDS.contains(&word.as_str()) {
                Class::Keyword
            } else if chars.get(i) == Some(&'(') || chars.get(i) == Some(&'!') {
                Class::Function
            } else if c.is_uppercase() {
                Class::Type
            } else {
                Class::Text
            };
            push(&mut out, col, i - start, class);
            col += i - start;
            continue;
        }
        push(&mut out, col, 1, Class::Text);
        col += 1;
        i += 1;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classifies_common_tokens() {
        let l = line("    let x = foo(\"hi\"); // note", 96);
        let classes: Vec<Class> = l.iter().map(|t| t.2).collect();
        assert_eq!(l[0].0, 4);
        assert_eq!(classes.first(), Some(&Class::Keyword));
        assert!(classes.contains(&Class::Function));
        assert!(classes.contains(&Class::String));
        assert_eq!(classes.last(), Some(&Class::Comment));
    }

    #[test]
    fn long_lines_are_cut_at_the_width() {
        let l = line(&"x".repeat(500), 96);
        assert_eq!(l, vec![(0, 96, Class::Text)]);
    }
}
