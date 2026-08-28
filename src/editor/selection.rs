/// エディタの選択範囲およびマルチカーソル選択の管理モジュール

use crate::editor::buffer::point::Point;

/// テキスト選択範囲
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Selection {
    /// 選択開始位置 (アンカー)
    pub start: Point,
    /// 選択終了位置 (アクティブ/ヘッド)
    pub end: Point,
    /// 選択方向が逆（ヘッドがアンカーより前）かどうか
    pub reversed: bool,
    /// 上下移動時に維持したい目標列（行末移動などで列が潰れた場合の復元用）
    pub goal_column: Option<usize>,
}

impl Selection {
    /// 単一カーソル位置（範囲なし）から選択を作成
    pub fn point(point: Point) -> Self {
        Self {
            start: point,
            end: point,
            reversed: false,
            goal_column: Some(point.column),
        }
    }

    /// 開始位置と終了位置を指定して作成
    pub fn new(start: Point, end: Point) -> Self {
        let reversed = start > end;
        let (head, anchor) = if reversed { (start, end) } else { (end, start) };
        Self {
            start: anchor,
            end: head,
            reversed,
            goal_column: Some(head.column),
        }
    }

    /// 選択範囲が空（カーソルのみ）かどうか
    pub fn is_empty(&self) -> bool {
        self.start == self.end
    }

    /// 正規化された最小位置（常に前）
    pub fn min(&self) -> Point {
        if self.start <= self.end {
            self.start
        } else {
            self.end
        }
    }

    /// 正規化された最大位置（常に後）
    pub fn max(&self) -> Point {
        if self.start >= self.end {
            self.start
        } else {
            self.end
        }
    }

    /// カーソル（ヘッド）位置を取得
    pub fn head(&self) -> Point {
        if self.reversed {
            self.start
        } else {
            self.end
        }
    }

    /// アンカー位置を取得
    pub fn anchor(&self) -> Point {
        if self.reversed {
            self.end
        } else {
            self.start
        }
    }

    /// 他の選択範囲と重複または隣接しているか
    pub fn overlaps_or_adjacent(&self, other: &Self) -> bool {
        let min_a = self.min();
        let max_a = self.max();
        let min_b = other.min();
        let max_b = other.max();
        !(max_a < min_b || max_b < min_a)
    }

    /// 重複する2つの選択範囲を1つに統合
    pub fn merge(&self, other: &Self) -> Self {
        let min = self.min().min(other.min());
        let max = self.max().max(other.max());
        let reversed = self.reversed;
        if reversed {
            Self {
                start: max,
                end: min,
                reversed: true,
                goal_column: other.goal_column.or(self.goal_column),
            }
        } else {
            Self {
                start: min,
                end: max,
                reversed: false,
                goal_column: other.goal_column.or(self.goal_column),
            }
        }
    }
}

/// 複数の選択範囲のリストを重複解消・ソートして正規化する
pub fn normalize_selections(selections: &mut Vec<Selection>) {
    if selections.is_empty() {
        selections.push(Selection::point(Point::zero()));
        return;
    }

    selections.sort_by_key(|s| s.min());

    let mut merged = Vec::with_capacity(selections.len());
    let mut current = selections[0];

    for next in selections.iter().skip(1) {
        if current.overlaps_or_adjacent(next) {
            current = current.merge(next);
        } else {
            merged.push(current);
            current = *next;
        }
    }
    merged.push(current);
    *selections = merged;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_selection_normalize() {
        let mut selections = vec![
            Selection::new(Point::new(0, 5), Point::new(0, 10)),
            Selection::new(Point::new(0, 8), Point::new(0, 15)),
        ];

        normalize_selections(&mut selections);
        assert_eq!(selections.len(), 1);
        assert_eq!(selections[0].min(), Point::new(0, 5));
        assert_eq!(selections[0].max(), Point::new(0, 15));
    }
}
