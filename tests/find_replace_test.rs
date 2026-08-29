/// インライン検索・置換エンジンの統合テスト

use nucleus::editor::find_replace::FindReplaceState;

#[test]
fn test_find_replace_matching_and_navigation() {
    let mut state = FindReplaceState::new();
    let text = "fn calculate_total() -> i32 {\n    let total = 100;\n    total\n}";

    // 1. 大文字小文字区別なし検索
    state.query = "total".to_string();
    state.update_matches(text);
    assert_eq!(state.matches.len(), 3);
    assert_eq!(state.current_match_index, 0);

    // 2. ナビゲーション (次へ / 前へ)
    state.next_match();
    assert_eq!(state.current_match_index, 1);
    state.next_match();
    assert_eq!(state.current_match_index, 2);
    state.next_match();
    assert_eq!(state.current_match_index, 0); // ラップアラウンド

    state.prev_match();
    assert_eq!(state.current_match_index, 2);

    // 3. 単語単位マッチ
    state.whole_word = true;
    state.update_matches(text);
    assert_eq!(state.matches.len(), 2); // calculate_total は除外され、let total と total のみ一致
}
