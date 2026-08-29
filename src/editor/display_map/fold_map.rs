/// コード折りたたみ（Code Folding）範囲マッピングモジュール

/// 折りたたみ範囲情報
#[derive(Clone, Debug, PartialEq)]
pub struct FoldRange {
    pub start_row: usize,
    pub end_row: usize,
    pub is_folded: bool,
}

/// コード折りたたみ状態と表示行の変換マップ
#[derive(Clone, Debug, Default)]
pub struct FoldMap {
    pub folds: Vec<FoldRange>,
}

impl FoldMap {
    /// 新規 FoldMap の作成
    pub fn new() -> Self {
        Self::default()
    }

    /// 折りたたみ範囲の追加
    pub fn add_fold(&mut self, start_row: usize, end_row: usize) {
        if start_row < end_row {
            self.folds.push(FoldRange {
                start_row,
                end_row,
                is_folded: true,
            });
            self.folds.sort_by_key(|f| f.start_row);
        }
    }

    /// 指定行の折りたたみを解除
    pub fn unfold(&mut self, start_row: usize) {
        if let Some(pos) = self.folds.iter().position(|f| f.start_row == start_row) {
            self.folds.remove(pos);
        }
    }

    /// 指定行が折りたたまれて非表示になっているか判定
    pub fn is_row_hidden(&self, row: usize) -> bool {
        for fold in &self.folds {
            if fold.is_folded && row > fold.start_row && row <= fold.end_row {
                return true;
            }
        }
        false
    }

    /// バッファ行番号から表示行番号を計算
    pub fn buffer_to_display_row(&self, buffer_row: usize) -> usize {
        let mut hidden_count = 0;
        for r in 0..buffer_row {
            if self.is_row_hidden(r) {
                hidden_count += 1;
            }
        }
        buffer_row.saturating_sub(hidden_count)
    }

    /// 表示行番号からバッファ行番号を計算
    pub fn display_to_buffer_row(&self, display_row: usize, max_buffer_rows: usize) -> usize {
        let mut current_display = 0;
        for r in 0..max_buffer_rows {
            if !self.is_row_hidden(r) {
                if current_display == display_row {
                    return r;
                }
                current_display += 1;
            }
        }
        max_buffer_rows.saturating_sub(1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fold_map_basic() {
        let mut map = FoldMap::new();
        // 2行目から5行目を折りたたむ
        map.add_fold(2, 5);

        assert!(!map.is_row_hidden(0));
        assert!(!map.is_row_hidden(2)); // 開始行は表示される
        assert!(map.is_row_hidden(3));  // 内部行は隠れる
        assert!(map.is_row_hidden(5));  // 終了行は隠れる
        assert!(!map.is_row_hidden(6));

        assert_eq!(map.buffer_to_display_row(0), 0);
        assert_eq!(map.buffer_to_display_row(2), 2);
        assert_eq!(map.buffer_to_display_row(6), 3); // 3,4,5の3行が隠れているので 6 - 3 = 3

        assert_eq!(map.display_to_buffer_row(0, 10), 0);
        assert_eq!(map.display_to_buffer_row(2, 10), 2);
        assert_eq!(map.display_to_buffer_row(3, 10), 6);
    }
}
