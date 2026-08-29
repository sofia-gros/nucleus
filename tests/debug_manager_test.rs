/// DAP デバッグマネージャーの統合テスト

use nucleus::debug::DebugManager;

#[test]
fn test_debug_manager_breakpoints() {
    let mut mgr = DebugManager::new();
    let file = "src/main.rs";

    // 1. ブレークポイントの追加
    let added = mgr.toggle_breakpoint(file, 42);
    assert!(added);
    assert!(mgr.has_breakpoint(file, 42));
    assert!(!mgr.has_breakpoint(file, 43));

    // 2. ブレークポイントの解除
    let removed = mgr.toggle_breakpoint(file, 42);
    assert!(!removed);
    assert!(!mgr.has_breakpoint(file, 42));
}
