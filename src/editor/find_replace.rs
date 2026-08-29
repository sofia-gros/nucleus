/// エディタ内インライン検索・置換（Find & Replace）状態およびロジックモジュール

/// 検索・置換の一致箇所
#[derive(Clone, Debug, PartialEq)]
pub struct FindMatch {
    pub start_offset: usize,
    pub end_offset: usize,
    pub line_number: usize,
}

/// インライン検索・置換の状態
#[derive(Clone, Debug)]
pub struct FindReplaceState {
    pub is_open: bool,
    pub is_replace_open: bool,
    pub query: String,
    pub replace_text: String,
    pub case_sensitive: bool,
    pub whole_word: bool,
    pub matches: Vec<FindMatch>,
    pub current_match_index: usize,
}

impl Default for FindReplaceState {
    fn default() -> Self {
        Self::new()
    }
}

impl FindReplaceState {
    /// 新規作成
    pub fn new() -> Self {
        Self {
            is_open: false,
            is_replace_open: false,
            query: String::new(),
            replace_text: String::new(),
            case_sensitive: false,
            whole_word: false,
            matches: Vec::new(),
            current_match_index: 0,
        }
    }

    /// 検索バーを開く (Ctrl+F)
    pub fn open_find(&mut self) {
        self.is_open = true;
        self.is_replace_open = false;
    }

    /// 置換バーを開く (Ctrl+H)
    pub fn open_replace(&mut self) {
        self.is_open = true;
        self.is_replace_open = true;
    }

    /// 検索バーを閉じる
    pub fn close(&mut self) {
        self.is_open = false;
        self.is_replace_open = false;
        self.matches.clear();
        self.current_match_index = 0;
    }

    /// テキスト内の一致箇所を走査
    pub fn update_matches(&mut self, text: &str) {
        self.matches.clear();
        if self.query.is_empty() {
            self.current_match_index = 0;
            return;
        }

        let target_query = if self.case_sensitive {
            self.query.clone()
        } else {
            self.query.to_lowercase()
        };

        let mut current_offset = 0;
        for (line_idx, line) in text.lines().enumerate() {
            let search_line = if self.case_sensitive {
                line.to_string()
            } else {
                line.to_lowercase()
            };

            let mut start_pos = 0;
            while let Some(found) = search_line[start_pos..].find(&target_query) {
                let actual_start = start_pos + found;
                let actual_end = actual_start + target_query.len();

                let is_valid = if self.whole_word {
                    let is_word_char = |c: char| c.is_alphanumeric() || c == '_';
                    let prev_ok = actual_start == 0 || !search_line.chars().nth(actual_start - 1).map(is_word_char).unwrap_or(false);
                    let next_ok = actual_end >= search_line.len() || !search_line.chars().nth(actual_end).map(is_word_char).unwrap_or(false);
                    prev_ok && next_ok
                } else {
                    true
                };

                if is_valid {
                    self.matches.push(FindMatch {
                        start_offset: current_offset + actual_start,
                        end_offset: current_offset + actual_end,
                        line_number: line_idx + 1,
                    });
                }

                start_pos = actual_start + 1;
                if start_pos >= search_line.len() {
                    break;
                }
            }

            current_offset += line.len() + 1;
        }

        if self.matches.is_empty() {
            self.current_match_index = 0;
        } else if self.current_match_index >= self.matches.len() {
            self.current_match_index = 0;
        }
    }

    /// 次の一致へ進む
    pub fn next_match(&mut self) -> Option<&FindMatch> {
        if self.matches.is_empty() {
            return None;
        }
        self.current_match_index = (self.current_match_index + 1) % self.matches.len();
        self.matches.get(self.current_match_index)
    }

    /// 前の一致へ戻る
    pub fn prev_match(&mut self) -> Option<&FindMatch> {
        if self.matches.is_empty() {
            return None;
        }
        if self.current_match_index == 0 {
            self.current_match_index = self.matches.len() - 1;
        } else {
            self.current_match_index -= 1;
        }
        self.matches.get(self.current_match_index)
    }
}
