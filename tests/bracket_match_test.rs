/// 括弧ペアカラーリング & インデントガイドの統合テスト

use nucleus::editor::bracket_match::BracketMatchEngine;

#[test]
fn test_bracket_pairs_and_indent_guides() {
    let text = "fn main() {\n    if (true) {\n        let arr = [1, 2, 3];\n    }\n}";

    // 1. 括弧ペアの抽出 (main(), main{}, if(), let [], if{})
    let pairs = BracketMatchEngine::find_bracket_pairs(text);
    assert_eq!(pairs.len(), 5);

    // 2. インデントガイドの計算
    let guides = BracketMatchEngine::calculate_indent_guides(text, 4);
    assert_eq!(guides.len(), 3);
    assert_eq!(guides[0].level, 1);
    assert_eq!(guides[1].level, 2);
    assert_eq!(guides[2].level, 1);
}
