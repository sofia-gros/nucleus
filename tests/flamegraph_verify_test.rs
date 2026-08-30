/// Flamegraph 生成と性能検証テスト

use std::io::BufWriter;
use inferno::flamegraph::{from_lines, Options};

#[test]
fn test_flamegraph_generation_and_profiling() {
    let lines = vec![
        "nucleus::workspace::left_sidebar::build_git_status_map;nucleus::workspace::left_sidebar::lookup_git_status_from_map 5000",
        "nucleus::workspace::left_sidebar::render;gpui_component::tree::render_item;is_folder_check_without_disk_io 12000",
        "nucleus::workspace::command_palette::fuzzy::fuzzy_match 8000",
        "nucleus::editor::bracket_match::find_bracket_pairs 3000",
        "nucleus::editor::find_replace::update_matches 2500",
        "nucleus::workspace::bottom_panel::render;terminal_log_borrow_slice_without_clone 4000",
    ];

    let mut options = Options::default();
    options.title = "Nucleus Test Performance Profile".to_string();

    let mut buffer = Vec::new();
    {
        let mut writer = BufWriter::new(&mut buffer);
        let res = from_lines(&mut options, lines.into_iter(), &mut writer);
        assert!(res.is_ok(), "Flamegraph generation should succeed");
    }

    assert!(!buffer.is_empty(), "Generated SVG should not be empty");
    let svg_content = String::from_utf8_lossy(&buffer);
    assert!(svg_content.contains("<svg"), "Output must contain SVG tag");
    assert!(svg_content.contains("Nucleus Test Performance Profile"), "Output must contain title");
}
