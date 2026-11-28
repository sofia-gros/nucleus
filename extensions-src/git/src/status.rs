//! `git status --porcelain=v1 -b -z` の読み取り。

#[derive(Debug, Default, PartialEq)]
pub struct Status {
    pub branch: String,
    pub ahead: u32,
    pub behind: u32,
    pub staged: Vec<Entry>,
    pub unstaged: Vec<Entry>,
}

#[derive(Debug, PartialEq)]
pub struct Entry {
    pub path: String,
    /// M / A / D / R / C / U（未追跡）
    pub code: char,
}

impl Status {
    /// パスを `prefix`（リポジトリの一番上から見た、開いているフォルダ。`src/` など）からの相対にする。
    /// 開いているフォルダの外の変更は除く。
    pub fn relative_to(mut self, prefix: &str) -> Status {
        if prefix.is_empty() {
            return self;
        }
        let strip = |list: Vec<Entry>| -> Vec<Entry> {
            list.into_iter()
                .filter_map(|e| {
                    let path = e.path.strip_prefix(prefix)?.to_string();
                    Some(Entry { path, code: e.code })
                })
                .collect()
        };
        self.staged = strip(self.staged);
        self.unstaged = strip(self.unstaged);
        self
    }
}

pub fn parse(out: &str) -> Status {
    let mut st = Status::default();
    let mut fields = out.split('\0').filter(|s| !s.is_empty());
    while let Some(f) = fields.next() {
        if let Some(head) = f.strip_prefix("## ") {
            parse_branch(head, &mut st);
            continue;
        }
        if f.len() < 4 {
            continue;
        }
        let mut chars = f.chars();
        let x = chars.next().unwrap_or(' ');
        let y = chars.next().unwrap_or(' ');
        let path = f[3..].to_string();
        // 名前の変更とコピーは、次のフィールドに元の名前が入る。
        if matches!(x, 'R' | 'C') || matches!(y, 'R' | 'C') {
            fields.next();
        }
        if x == '?' {
            st.unstaged.push(Entry { path, code: 'U' });
            continue;
        }
        // 衝突（マージ中）: UU / AA / DD / AU / UA / DU / UD
        if x == 'U' || y == 'U' || (x == 'A' && y == 'A') || (x == 'D' && y == 'D') {
            st.unstaged.push(Entry { path, code: '!' });
            continue;
        }
        if x == '!' {
            continue;
        }
        if x != ' ' {
            st.staged.push(Entry {
                path: path.clone(),
                code: x,
            });
        }
        if y != ' ' {
            st.unstaged.push(Entry { path, code: y });
        }
    }
    st
}

fn parse_branch(head: &str, st: &mut Status) {
    // 例: "main...origin/main [ahead 1, behind 2]" / "No commits yet on main" / "HEAD (no branch)"
    let (names, rest) = match head.split_once(" [") {
        Some((n, r)) => (n, r.trim_end_matches(']')),
        None => (head, ""),
    };
    let local = names.split("...").next().unwrap_or(names);
    st.branch = local
        .strip_prefix("No commits yet on ")
        .or_else(|| local.strip_prefix("Initial commit on "))
        .unwrap_or(local)
        .to_string();
    for part in rest.split(", ") {
        if let Some(n) = part.strip_prefix("ahead ") {
            st.ahead = n.parse().unwrap_or(0);
        } else if let Some(n) = part.strip_prefix("behind ") {
            st.behind = n.parse().unwrap_or(0);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_branch_and_entries() {
        let out = "## main...origin/main [ahead 1, behind 2]\0M  src/a.rs\0 M b.txt\0MM c.rs\0?? new/d.rs\0R  e2.rs\0e.rs\0 D f.rs\0";
        let st = parse(out);
        assert_eq!(st.branch, "main");
        assert_eq!((st.ahead, st.behind), (1, 2));
        let staged: Vec<_> = st.staged.iter().map(|e| (e.path.as_str(), e.code)).collect();
        assert_eq!(staged, vec![("src/a.rs", 'M'), ("c.rs", 'M'), ("e2.rs", 'R')]);
        let unstaged: Vec<_> = st.unstaged.iter().map(|e| (e.path.as_str(), e.code)).collect();
        assert_eq!(
            unstaged,
            vec![("b.txt", 'M'), ("c.rs", 'M'), ("new/d.rs", 'U'), ("f.rs", 'D')]
        );
    }

    #[test]
    fn paths_become_relative_to_the_open_folder() {
        let st = parse("## main\0 M src/a.rs\0 M README.md\0").relative_to("src/");
        let paths: Vec<_> = st.unstaged.iter().map(|e| e.path.as_str()).collect();
        assert_eq!(paths, vec!["a.rs"]);
    }

    #[test]
    fn conflicts_are_not_untracked() {
        let st = parse("## main\0UU a.rs\0?? b.rs\0");
        let codes: Vec<_> = st.unstaged.iter().map(|e| (e.path.as_str(), e.code)).collect();
        assert_eq!(codes, vec![("a.rs", '!'), ("b.rs", 'U')]);
        assert!(st.staged.is_empty());
    }

    #[test]
    fn parses_a_fresh_repository() {
        let st = parse("## No commits yet on main\0A  a.rs\0");
        assert_eq!(st.branch, "main");
        assert_eq!(st.staged.len(), 1);
    }
}
