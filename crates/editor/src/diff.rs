//! 行単位の差分（Diff 表示、Revert Hunk、Stage Hunk 用のパッチ）。gpui に依存しない。

use similar::{ChangeTag, TextDiff};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LineKind {
    Context,
    Added,
    Removed,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DiffLine {
    pub kind: LineKind,
    /// 1 始まりの行番号（古い側 / 新しい側）。
    pub old_no: Option<u32>,
    pub new_no: Option<u32>,
    /// 改行を除いた行。
    pub text: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Hunk {
    /// 0 始まりの開始行と行数（前後の文脈を含む）。
    pub old_start: usize,
    pub old_len: usize,
    pub new_start: usize,
    pub new_len: usize,
    pub lines: Vec<DiffLine>,
}

impl Hunk {
    /// `@@ -22,8 +22,9 @@` の形の見出し。
    pub fn header(&self) -> String {
        format!(
            "@@ -{},{} +{},{} @@",
            start_no(self.old_start, self.old_len),
            self.old_len,
            start_no(self.new_start, self.new_len),
            self.new_len
        )
    }

    pub fn added(&self) -> usize {
        self.lines.iter().filter(|l| l.kind == LineKind::Added).count()
    }

    pub fn removed(&self) -> usize {
        self.lines.iter().filter(|l| l.kind == LineKind::Removed).count()
    }
}

/// unified diff の行番号（長さ 0 なら直前の行）。
fn start_no(start: usize, len: usize) -> usize {
    if len == 0 { start } else { start + 1 }
}

/// `old` から `new` への差分を、前後 `context` 行の文脈つきの塊に分ける。
pub fn compute(old: &str, new: &str, context: usize) -> Vec<Hunk> {
    let diff = TextDiff::from_lines(old, new);
    let mut hunks = Vec::new();
    for group in diff.grouped_ops(context) {
        let (Some(first), Some(last)) = (group.first(), group.last()) else {
            continue;
        };
        let old_start = first.old_range().start;
        let new_start = first.new_range().start;
        let old_len = last.old_range().end - old_start;
        let new_len = last.new_range().end - new_start;
        let mut lines = Vec::new();
        for op in &group {
            for change in diff.iter_changes(op) {
                let kind = match change.tag() {
                    ChangeTag::Equal => LineKind::Context,
                    ChangeTag::Insert => LineKind::Added,
                    ChangeTag::Delete => LineKind::Removed,
                };
                let text = change.value().trim_end_matches(['\n', '\r']).to_string();
                lines.push(DiffLine {
                    kind,
                    old_no: change.old_index().map(|i| i as u32 + 1),
                    new_no: change.new_index().map(|i| i as u32 + 1),
                    text,
                });
            }
        }
        hunks.push(Hunk {
            old_start,
            old_len,
            new_start,
            new_len,
            lines,
        });
    }
    hunks
}

/// 改行コードを保ったまま行に分ける。
fn split_lines(text: &str) -> Vec<&str> {
    text.split_inclusive('\n').collect()
}

/// `new` の中の `hunk` を元（old 側）に戻したテキスト。
pub fn revert_hunk(new: &str, old: &str, hunk: &Hunk) -> String {
    let new_lines = split_lines(new);
    let old_lines = split_lines(old);
    let mut out = String::with_capacity(new.len());
    for l in &new_lines[..hunk.new_start.min(new_lines.len())] {
        out.push_str(l);
    }
    let old_end = (hunk.old_start + hunk.old_len).min(old_lines.len());
    for l in &old_lines[hunk.old_start.min(old_end)..old_end] {
        out.push_str(l);
    }
    let new_end = (hunk.new_start + hunk.new_len).min(new_lines.len());
    for l in &new_lines[new_end..] {
        out.push_str(l);
    }
    out
}

/// `git apply --cached` に渡す、1つの塊だけのパッチ。`path` はリポジトリからの相対パス。
pub fn hunk_patch(path: &str, hunk: &Hunk, old_ends_with_newline: bool, new_ends_with_newline: bool) -> String {
    let mut p = format!(
        "diff --git a/{path} b/{path}\n--- a/{path}\n+++ b/{path}\n{}\n",
        hunk.header()
    );
    let last_old = hunk.lines.iter().rposition(|l| l.kind != LineKind::Added);
    let last_new = hunk.lines.iter().rposition(|l| l.kind != LineKind::Removed);
    for (i, l) in hunk.lines.iter().enumerate() {
        let sign = match l.kind {
            LineKind::Context => ' ',
            LineKind::Added => '+',
            LineKind::Removed => '-',
        };
        p.push(sign);
        p.push_str(&l.text);
        p.push('\n');
        // ファイルの最後の行に改行がないとき。
        let at_old_eof = Some(i) == last_old && l.old_no.is_some_and(|n| n as usize == hunk.old_start + hunk.old_len);
        let at_new_eof = Some(i) == last_new && l.new_no.is_some_and(|n| n as usize == hunk.new_start + hunk.new_len);
        let missing = match l.kind {
            LineKind::Removed => at_old_eof && !old_ends_with_newline,
            LineKind::Added => at_new_eof && !new_ends_with_newline,
            LineKind::Context => (at_old_eof && !old_ends_with_newline) || (at_new_eof && !new_ends_with_newline),
        };
        if missing {
            p.push_str("\\ No newline at end of file\n");
        }
    }
    p
}

#[cfg(test)]
mod tests {
    use super::*;

    const OLD: &str = "use a;\nuse b;\n\nfn one() {}\nfn two() {}\nfn three() {}\nfn four() {}\nfn five() {}\nfn six() {}\nfn seven() {}\nfn eight() {}\n";

    fn new_text() -> String {
        OLD.replace("use a;\n", "use z;\n")
            .replace("fn eight() {}\n", "fn eight() {}\nfn nine() {}\n")
    }

    #[test]
    fn hunks_have_context_and_line_numbers() {
        let hunks = compute(OLD, &new_text(), 3);
        assert_eq!(hunks.len(), 2);
        let h = &hunks[0];
        assert_eq!(h.header(), "@@ -1,4 +1,4 @@");
        assert_eq!((h.added(), h.removed()), (1, 1));
        assert_eq!(h.lines[0].kind, LineKind::Removed);
        assert_eq!(h.lines[0].text, "use a;");
        assert_eq!(h.lines[1].new_no, Some(1));
        let h2 = &hunks[1];
        assert_eq!(h2.header(), "@@ -9,3 +9,4 @@");
        assert_eq!(h2.lines.last().unwrap().text, "fn nine() {}");
    }

    #[test]
    fn reverting_a_hunk_restores_only_that_part() {
        let new = new_text();
        let hunks = compute(OLD, &new, 3);
        let reverted = revert_hunk(&new, OLD, &hunks[1]);
        assert!(reverted.starts_with("use z;\n"));
        assert!(!reverted.contains("nine"));
        let both = revert_hunk(&reverted, OLD, &compute(OLD, &reverted, 3)[0]);
        assert_eq!(both, OLD);
    }

    #[test]
    fn patches_mark_a_missing_final_newline() {
        let old = "a\nb";
        let new = "a\nc";
        let hunks = compute(old, new, 3);
        let p = hunk_patch("x.txt", &hunks[0], false, false);
        assert!(
            p.contains("-b\n\\ No newline at end of file\n+c\n\\ No newline at end of file\n"),
            "{p}"
        );
    }
}
