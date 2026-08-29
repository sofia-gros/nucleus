/// 括弧ペアカラーリングおよびインデントガイド線の計算モジュール

/// 括弧ペア情報
#[derive(Clone, Debug, PartialEq)]
pub struct BracketPair {
    pub open_offset: usize,
    pub close_offset: usize,
    pub level: usize,
}

/// 各行のインデントガイド情報
#[derive(Clone, Debug, PartialEq)]
pub struct IndentGuide {
    pub line_number: usize,
    pub level: usize,
}

/// 括弧ペアとインデントの解析器
pub struct BracketMatchEngine;

impl BracketMatchEngine {
    /// テキスト全体の括弧ペアを解析
    pub fn find_bracket_pairs(text: &str) -> Vec<BracketPair> {
        let mut pairs = Vec::new();
        let mut stack: Vec<(char, usize, usize)> = Vec::new(); // (char, offset, level)

        for (offset, ch) in text.char_indices() {
            match ch {
                '{' | '(' | '[' => {
                    let level = stack.len();
                    stack.push((ch, offset, level));
                }
                '}' => {
                    if let Some((open_ch, open_offset, level)) = stack.pop() {
                        if open_ch == '{' {
                            pairs.push(BracketPair { open_offset, close_offset: offset, level });
                        }
                    }
                }
                ')' => {
                    if let Some((open_ch, open_offset, level)) = stack.pop() {
                        if open_ch == '(' {
                            pairs.push(BracketPair { open_offset, close_offset: offset, level });
                        }
                    }
                }
                ']' => {
                    if let Some((open_ch, open_offset, level)) = stack.pop() {
                        if open_ch == '[' {
                            pairs.push(BracketPair { open_offset, close_offset: offset, level });
                        }
                    }
                }
                _ => {}
            }
        }
        pairs
    }

    /// 各行のインデントレベルを計算
    pub fn calculate_indent_guides(text: &str, tab_size: usize) -> Vec<IndentGuide> {
        let mut guides = Vec::new();
        for (line_idx, line) in text.lines().enumerate() {
            let mut spaces = 0;
            for ch in line.chars() {
                if ch == ' ' {
                    spaces += 1;
                } else if ch == '\t' {
                    spaces += tab_size;
                } else {
                    break;
                }
            }
            let level = spaces / tab_size.max(1);
            if level > 0 {
                guides.push(IndentGuide {
                    line_number: line_idx + 1,
                    level,
                });
            }
        }
        guides
    }

    /// 階層に応じたカラーインデックスを取得 (0..3)
    pub fn level_to_color_index(level: usize) -> usize {
        level % 3
    }
}
