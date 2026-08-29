/// テキストのソフトラップ（行折り返し）計算モジュール

use crate::editor::buffer::point::Point;

/// ソフトラップされた行の情報
#[derive(Clone, Debug, PartialEq)]
pub struct WrappedLine {
    pub buffer_row: usize,
    pub start_col: usize,
    pub end_col: usize,
}

/// ソフトラップ座標変換マップ
#[derive(Clone, Debug)]
pub struct WrapMap {
    pub wrap_width: usize,
    pub wrapped_lines: Vec<WrappedLine>,
}

impl WrapMap {
    /// 新規 WrapMap の作成
    pub fn new(wrap_width: usize) -> Self {
        Self {
            wrap_width,
            wrapped_lines: Vec::new(),
        }
    }

    /// バッファテキスト全体のソフトラップ行を計算
    pub fn compute(&mut self, text: &str) {
        self.wrapped_lines.clear();
        if self.wrap_width == 0 {
            return;
        }

        for (row, line) in text.lines().enumerate() {
            let char_count = line.chars().count();
            if char_count == 0 {
                self.wrapped_lines.push(WrappedLine {
                    buffer_row: row,
                    start_col: 0,
                    end_col: 0,
                });
            } else {
                let mut start = 0;
                while start < char_count {
                    let end = (start + self.wrap_width).min(char_count);
                    self.wrapped_lines.push(WrappedLine {
                        buffer_row: row,
                        start_col: start,
                        end_col: end,
                    });
                    start = end;
                }
            }
        }
    }

    /// バッファ論理座標 Point(row, col) から表示行インデックスを取得
    pub fn buffer_to_display(&self, point: Point) -> usize {
        for (display_row, line) in self.wrapped_lines.iter().enumerate() {
            if line.buffer_row == point.row && point.column >= line.start_col && point.column <= line.end_col {
                return display_row;
            }
        }
        self.wrapped_lines.len().saturating_sub(1)
    }

    /// 表示行インデックスからバッファ論理行番号を取得
    pub fn display_to_buffer(&self, display_row: usize) -> Option<Point> {
        self.wrapped_lines.get(display_row).map(|line| Point::new(line.buffer_row, line.start_col))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wrap_map_basic() {
        let mut map = WrapMap::new(10);
        let text = "1234567890abcdef"; // 16文字 -> 10 + 6
        map.compute(text);

        assert_eq!(map.wrapped_lines.len(), 2);
        assert_eq!(map.wrapped_lines[0].start_col, 0);
        assert_eq!(map.wrapped_lines[0].end_col, 10);
        assert_eq!(map.wrapped_lines[1].start_col, 10);
        assert_eq!(map.wrapped_lines[1].end_col, 16);

        assert_eq!(map.buffer_to_display(Point::new(0, 5)), 0);
        assert_eq!(map.buffer_to_display(Point::new(0, 12)), 1);
    }
}
