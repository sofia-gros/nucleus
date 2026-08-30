/// ターミナルのスクロールオフセット計算の検証テスト

pub fn get_visible_terminal_lines<'a>(
    all_lines: &'a [String],
    view_capacity: usize,
    scroll_offset: usize,
) -> &'a [String] {
    let total = all_lines.len();
    if total <= view_capacity {
        return all_lines;
    }

    let max_offset = total.saturating_sub(view_capacity);
    let effective_offset = scroll_offset.min(max_offset);
    let end = total - effective_offset;
    let start = end.saturating_sub(view_capacity);

    &all_lines[start..end]
}

#[test]
fn test_terminal_scroll_offset_behavior() {
    let lines: Vec<String> = (0..100).map(|i| format!("Line {}", i)).collect();
    let capacity = 30;

    // 1. 最下部 (scroll_offset = 0): Line 70..Line 99 の 30 行
    let bottom_view = get_visible_terminal_lines(&lines, capacity, 0);
    assert_eq!(bottom_view.len(), 30);
    assert_eq!(bottom_view[0], "Line 70");
    assert_eq!(bottom_view[29], "Line 99");

    // 2. 10行上にスクロール (scroll_offset = 10): Line 60..Line 89
    let scrolled_view = get_visible_terminal_lines(&lines, capacity, 10);
    assert_eq!(scrolled_view.len(), 30);
    assert_eq!(scrolled_view[0], "Line 60");
    assert_eq!(scrolled_view[29], "Line 89");

    // 3. 最上部までスクロール (scroll_offset = 100): Line 0..Line 29
    let top_view = get_visible_terminal_lines(&lines, capacity, 100);
    assert_eq!(top_view.len(), 30);
    assert_eq!(top_view[0], "Line 0");
    assert_eq!(top_view[29], "Line 29");
}
