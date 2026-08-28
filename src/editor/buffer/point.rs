/// テキストバッファ内の2次元座標およびオフセット表現
use std::cmp::Ordering;

/// バッファ内の行・列座標（0-indexed）
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Point {
    /// 0から始まる行番号
    pub row: usize,
    /// 0から始まる列（文字単位）番号
    pub column: usize,
}

impl Point {
    /// 新しい `Point` を作成
    pub const fn new(row: usize, column: usize) -> Self {
        Self { row, column }
    }

    /// 原点 (0, 0)
    pub const fn zero() -> Self {
        Self { row: 0, column: 0 }
    }
}

impl PartialOrd for Point {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Point {
    fn cmp(&self, other: &Self) -> Ordering {
        match self.row.cmp(&other.row) {
            Ordering::Equal => self.column.cmp(&other.column),
            ord => ord,
        }
    }
}

/// バッファの変更に追従する安定アンカー（将来のCRDT・マーカー用）
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Anchor {
    /// 文字オフセット
    pub offset: usize,
    /// バッファの生成バージョン
    pub version: usize,
    /// 挿入時に前進するかどうか
    pub bias_right: bool,
}

impl Anchor {
    /// 新しいアンカーを作成
    pub fn new(offset: usize, version: usize, bias_right: bool) -> Self {
        Self {
            offset,
            version,
            bias_right,
        }
    }
}
