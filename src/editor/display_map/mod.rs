/// バッファテキストから画面表示用テキストへの変換パイプライン（DisplayMap）

use crate::editor::buffer::point::Point;
use crate::editor::buffer::TextBuffer;

/// 画面上の表示座標（タブ展開やラップを反映した座標）
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct DisplayPoint {
    /// 表示行番号（0-indexed）
    pub row: usize,
    /// 表示列番号（0-indexed）
    pub column: usize,
}

impl DisplayPoint {
    /// 新しい表示座標を作成
    pub const fn new(row: usize, column: usize) -> Self {
        Self { row, column }
    }
}

/// DisplayMap の設定オプション
#[derive(Clone, Debug)]
pub struct DisplayOptions {
    /// タブ幅（スペース換算数）
    pub tab_size: usize,
    /// ソフトラップ有効フラグ
    pub soft_wrap: bool,
    /// ソフトラップの折り返し文字幅（Noneの場合はコンテナ幅依存）
    pub wrap_width: Option<usize>,
}

impl Default for DisplayOptions {
    fn default() -> Self {
        Self {
            tab_size: 4,
            soft_wrap: false,
            wrap_width: None,
        }
    }
}

/// 画面描画用パイプラインを統合する DisplayMap
#[derive(Clone, Debug)]
pub struct DisplayMap {
    /// オプション設定
    pub options: DisplayOptions,
    /// キャッシュされたバッファバージョン
    pub buffer_version: usize,
}

impl DisplayMap {
    /// 新しい DisplayMap を作成
    pub fn new() -> Self {
        Self {
            options: DisplayOptions::default(),
            buffer_version: 0,
        }
    }

    /// 設定を指定して作成
    pub fn with_options(options: DisplayOptions) -> Self {
        Self {
            options,
            buffer_version: 0,
        }
    }

    /// バッファ座標 `Point` を表示座標 `DisplayPoint` に変換（タブ展開等を考慮）
    pub fn point_to_display_point(&self, buffer: &TextBuffer, point: Point) -> DisplayPoint {
        let point = buffer.clip_point(point);
        if let Some(line) = buffer.line_to_string(point.row) {
            let mut display_col = 0;
            for (char_idx, ch) in line.chars().enumerate() {
                if char_idx >= point.column {
                    break;
                }
                if ch == '\t' {
                    let tab_stop = self.options.tab_size - (display_col % self.options.tab_size);
                    display_col += tab_stop;
                } else {
                    display_col += 1;
                }
            }
            DisplayPoint::new(point.row, display_col)
        } else {
            DisplayPoint::new(point.row, point.column)
        }
    }

    /// 表示座標 `DisplayPoint` をバッファ論理座標 `Point` に変換
    pub fn display_point_to_point(&self, buffer: &TextBuffer, display_point: DisplayPoint) -> Point {
        let row = display_point.row.min(buffer.len_lines().saturating_sub(1));
        if let Some(line) = buffer.line_to_string(row) {
            let mut current_display_col = 0;
            for (char_idx, ch) in line.chars().enumerate() {
                if current_display_col >= display_point.column {
                    return Point::new(row, char_idx);
                }
                if ch == '\t' {
                    let tab_stop = self.options.tab_size - (current_display_col % self.options.tab_size);
                    if current_display_col + tab_stop > display_point.column {
                        return Point::new(row, char_idx);
                    }
                    current_display_col += tab_stop;
                } else {
                    current_display_col += 1;
                }
            }
            Point::new(row, buffer.line_len(row))
        } else {
            Point::new(row, 0)
        }
    }

    /// 指定行の展開済み表示文字列を取得
    pub fn get_display_line(&self, buffer: &TextBuffer, row: usize) -> Option<String> {
        let raw = buffer.line_to_string(row)?;
        let mut display_line = String::with_capacity(raw.len());
        let mut col = 0;
        for ch in raw.chars() {
            if ch == '\t' {
                let tab_stop = self.options.tab_size - (col % self.options.tab_size);
                for _ in 0..tab_stop {
                    display_line.push(' ');
                }
                col += tab_stop;
            } else {
                display_line.push(ch);
                col += 1;
            }
        }
        Some(display_line)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_display_map_tab_expansion() {
        let buffer = TextBuffer::new("\thello");
        let display_map = DisplayMap::new();

        let display_line = display_map.get_display_line(&buffer, 0).unwrap();
        assert_eq!(display_line, "    hello"); // 4 spaces for tab

        let pt = Point::new(0, 1);
        let display_pt = display_map.point_to_display_point(&buffer, pt);
        assert_eq!(display_pt, DisplayPoint::new(0, 4));

        let back_pt = display_map.display_point_to_point(&buffer, display_pt);
        assert_eq!(back_pt, pt);
    }
}
