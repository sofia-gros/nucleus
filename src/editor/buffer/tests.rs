#[cfg(test)]
mod tests {
    use crate::editor::buffer::point::Point;
    use crate::editor::buffer::TextBuffer;

    #[test]
    fn test_point_ordering() {
        let p1 = Point::new(0, 5);
        let p2 = Point::new(1, 0);
        let p3 = Point::new(1, 2);

        assert!(p1 < p2);
        assert!(p2 < p3);
        assert_eq!(p1, Point::new(0, 5));
    }

    #[test]
    fn test_buffer_insert_delete() {
        let mut buf = TextBuffer::new("Hello World");
        assert_eq!(buf.len_chars(), 11);
        assert_eq!(buf.len_lines(), 1);

        // 挿入テスト
        let next_pt = buf.insert(Point::new(0, 5), ", Beautiful");
        assert_eq!(buf.to_string(), "Hello, Beautiful World");
        assert_eq!(next_pt, Point::new(0, 16));
        assert!(buf.is_dirty);

        // 削除テスト
        let deleted = buf.delete(Point::new(0, 5), Point::new(0, 16));
        assert_eq!(deleted, ", Beautiful");
        assert_eq!(buf.to_string(), "Hello World");

        // 複数行テキストの挿入
        buf.insert(Point::new(0, 5), "\nNew Line");
        assert_eq!(buf.len_lines(), 2);
        assert_eq!(buf.line_to_string(0).unwrap(), "Hello");
        assert_eq!(buf.line_to_string(1).unwrap(), "New Line World");
    }

    #[test]
    fn test_buffer_undo_redo() {
        let mut buf = TextBuffer::new("Initial");

        buf.insert(Point::new(0, 7), " Text");
        assert_eq!(buf.to_string(), "Initial Text");

        // Undo
        assert!(buf.undo());
        assert_eq!(buf.to_string(), "Initial");

        // Redo
        assert!(buf.redo());
        assert_eq!(buf.to_string(), "Initial Text");

        // さらなる編集
        buf.insert(Point::new(0, 12), " Added");
        assert_eq!(buf.to_string(), "Initial Text Added");

        assert!(buf.undo());
        assert_eq!(buf.to_string(), "Initial Text");
        assert!(buf.undo());
        assert_eq!(buf.to_string(), "Initial");
    }

    #[test]
    fn test_point_offset_conversion() {
        let buf = TextBuffer::new("Line1\nLine2\nLine3");
        assert_eq!(buf.len_lines(), 3);

        let pt = Point::new(1, 2);
        let offset = buf.point_to_offset(pt);
        assert_eq!(offset, 8); // "Line1\n" is 6 chars, plus 2 = 8

        let converted_pt = buf.offset_to_point(offset);
        assert_eq!(converted_pt, pt);
    }
}
