/// ファジーマッチングおよびスコアリングアルゴリズムモジュール

/// ファジーマッチの結果
#[derive(Debug, Clone, PartialEq)]
pub struct FuzzyMatch {
    pub score: i32,
    pub match_indices: Vec<usize>,
}

/// クエリ文字列と対象文字列のファジーマッチングを計算
///
/// 一致しない場合は `None` を返し、一致した場合はスコアが高いほど良いマッチとなります。
pub fn fuzzy_match(query: &str, target: &str) -> Option<FuzzyMatch> {
    if query.is_empty() {
        return Some(FuzzyMatch {
            score: 0,
            match_indices: Vec::new(),
        });
    }

    let query_chars: Vec<char> = query.to_lowercase().chars().collect();
    let target_chars: Vec<char> = target.to_lowercase().chars().collect();
    let original_target_chars: Vec<char> = target.chars().collect();

    if query_chars.len() > target_chars.len() {
        return None;
    }

    let mut query_idx = 0;
    let mut match_indices = Vec::with_capacity(query_chars.len());
    let mut score = 0;
    let mut prev_match_idx: Option<usize> = None;

    for (target_idx, &t_char) in target_chars.iter().enumerate() {
        if query_idx < query_chars.len() && t_char == query_chars[query_idx] {
            match_indices.push(target_idx);

            // スコアリングボーナス計算
            let mut char_score = 10;

            // 1. 連続一致ボーナス
            if let Some(prev) = prev_match_idx {
                if target_idx == prev + 1 {
                    char_score += 15;
                }
            }

            // 2. 単語先頭・境界ボーナス (/, \, _, -, ., 空白の直後)
            if target_idx == 0 {
                char_score += 20;
            } else {
                let prev_char = original_target_chars[target_idx - 1];
                if prev_char == '/' || prev_char == '\\' || prev_char == '_' || prev_char == '-' || prev_char == '.' || prev_char == ' ' {
                    char_score += 20;
                } else if original_target_chars[target_idx].is_uppercase() && !prev_char.is_uppercase() {
                    // キャメルケース境界
                    char_score += 15;
                }
            }

            score += char_score;
            prev_match_idx = Some(target_idx);
            query_idx += 1;
        }
    }

    if query_idx == query_chars.len() {
        // 完全一致へのボーナス
        if query_chars.len() == target_chars.len() {
            score += 50;
        }
        Some(FuzzyMatch {
            score,
            match_indices,
        })
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fuzzy_match_exact() {
        let res = fuzzy_match("main", "main.rs");
        assert!(res.is_some());
        let m = res.unwrap();
        assert_eq!(m.match_indices, vec![0, 1, 2, 3]);
    }

    #[test]
    fn test_fuzzy_match_acronym() {
        let res = fuzzy_match("cm", "Cargo.toml");
        assert!(res.is_some());
    }

    #[test]
    fn test_fuzzy_match_no_match() {
        let res = fuzzy_match("xyz", "main.rs");
        assert!(res.is_none());
    }
}
