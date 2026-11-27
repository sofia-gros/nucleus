//! 設定ファイルの値を1つだけ書き換える（コメントや書式は残す）。パレットでテーマや言語を選んだときに使う。

use std::path::Path;

/// トップレベルの `key` を `value`（JSON）にする。キーがなければ先頭に足す。バックグラウンドで呼ぶこと。
pub fn set_value(path: &Path, key: &str, value: &serde_json::Value) -> std::io::Result<()> {
    let text = match std::fs::read_to_string(path) {
        Ok(t) => t,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => crate::USER_TEMPLATE.to_string(),
        Err(e) => return Err(e),
    };
    let updated = set_in_text(&text, key, value);
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let tmp = path.with_extension("json.tmp");
    std::fs::write(&tmp, updated)?;
    std::fs::rename(tmp, path)
}

pub fn set_in_text(text: &str, key: &str, value: &serde_json::Value) -> String {
    let value = value.to_string();
    let needle = format!("\"{key}\"");
    let mut depth = 0i32;
    let mut out = Vec::new();
    let mut done = false;
    for line in text.lines() {
        let trimmed = line.trim_start();
        let is_comment = trimmed.starts_with("//");
        // コメントでない、深さ 1（トップレベル）の 1 行の値だけを書き換える。
        if !done && !is_comment && depth == 1 && trimmed.starts_with(&needle) {
            let rest = trimmed[needle.len()..].trim_start();
            if let Some(after_colon) = rest.strip_prefix(':') {
                let v = after_colon.trim();
                let single_line = !v.starts_with('{') && !v.starts_with('[');
                if single_line {
                    let indent = &line[..line.len() - trimmed.len()];
                    let comma = if v.trim_end().ends_with(',') { "," } else { "" };
                    out.push(format!("{indent}{needle}: {value}{comma}"));
                    done = true;
                    continue;
                }
            }
        }
        if !is_comment {
            for c in strip_strings(line).chars() {
                match c {
                    '{' | '[' => depth += 1,
                    '}' | ']' => depth -= 1,
                    _ => {}
                }
            }
        }
        out.push(line.to_string());
    }
    if !done {
        // 最初の `{` の直後に足す（後ろに値が続いてもよいように末尾にカンマを付ける。JSON with comments として読む）。
        if let Some(pos) = out
            .iter()
            .position(|l| !l.trim_start().starts_with("//") && l.contains('{'))
        {
            let line = out[pos].clone();
            let i = line.find('{').expect("contains {");
            let (head, tail) = line.split_at(i + 1);
            let mut lines = vec![head.to_string(), format!("  {needle}: {value},")];
            if !tail.trim().is_empty() {
                lines.push(tail.to_string());
            }
            out.splice(pos..=pos, lines);
        } else {
            out.push(format!("{{\n  {needle}: {value}\n}}"));
        }
    }
    let mut s = out.join("\n");
    if text.ends_with('\n') || !done {
        s.push('\n');
    }
    s
}

/// 文字列リテラルと行末コメントを取り除いた行（括弧の深さを数えるため）。
fn strip_strings(line: &str) -> String {
    let mut out = String::new();
    let mut in_str = false;
    let mut chars = line.chars().peekable();
    while let Some(c) = chars.next() {
        if in_str {
            if c == '\\' {
                chars.next();
            } else if c == '"' {
                in_str = false;
            }
            continue;
        }
        if c == '"' {
            in_str = true;
        } else if c == '/' && chars.peek() == Some(&'/') {
            break;
        } else {
            out.push(c);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn parse(s: &str) -> serde_json::Value {
        serde_json::from_str(&crate::strip_comments(s)).unwrap()
    }

    #[test]
    fn replaces_existing_top_level_value_keeping_comments() {
        let src = "// mine\n{\n  // theme\n  \"theme\": \"Nucleus One Dark\",\n  \"editor\": { \"theme\": 1 }\n}\n";
        let out = set_in_text(src, "theme", &json!("Nord"));
        assert!(out.contains("// mine") && out.contains("// theme"));
        let v = parse(&out);
        assert_eq!(v["theme"], "Nord");
        assert_eq!(v["editor"]["theme"], 1, "nested keys are untouched");
    }

    #[test]
    fn adds_missing_keys_to_the_template() {
        let out = set_in_text(crate::USER_TEMPLATE, "language", &json!("ja"));
        assert_eq!(parse(&out)["language"], "ja");
        let out = set_in_text("{}", "theme", &json!("Nord"));
        assert_eq!(parse(&out)["theme"], "Nord");
    }

    #[test]
    fn ignores_commented_out_keys() {
        let src = "{\n  // \"theme\": \"x\",\n  \"a\": 1\n}\n";
        let out = set_in_text(src, "theme", &json!("Nord"));
        assert!(out.contains("// \"theme\": \"x\""));
        assert_eq!(parse(&out)["theme"], "Nord");
        assert_eq!(parse(&out)["a"], 1);
    }

    #[test]
    fn writes_files_atomically() {
        let d = tempfile::tempdir().unwrap();
        let p = d.path().join("nucleus/settings.json");
        set_value(&p, "theme", &json!("Nord")).unwrap();
        assert_eq!(parse(&std::fs::read_to_string(&p).unwrap())["theme"], "Nord");
    }
}
