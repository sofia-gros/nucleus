//! Nucleus の起動（docs/ARCHITECTURE.md §5）。
//!
//! 最初のフレームは拡張を待たずに描く: テーマはキャッシュ（なければ同梱テーマ）、
//! 文言は同梱の Core 文言で即座に有効にしてからウィンドウを開く。

use gpui_kit::App;

fn main() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    gpui_kit::application()
        .with_assets(workspace::Assets)
        .run(|cx: &mut App| {
            gpui_kit::init(cx);
            if let Err(e) = cx.text_system().add_fonts(workspace::fonts()) {
                log::error!("同梱フォントを読み込めなかった: {e}");
            }
            theme::init(theme::Mode::Dark, cx);
            i18n::init(i18n::detect_language(), cx);
            workspace::init(cx);
            for e in workspace::init_settings(cx) {
                log::error!("設定を反映できなかった: {e}");
            }

            let paths = std::env::args_os().skip(1).map(std::path::PathBuf::from).collect();
            if let Err(e) = workspace::open_main_window(paths, cx) {
                log::error!("ウィンドウを開けなかった: {e}");
                cx.quit();
            }
            cx.activate(true);
        });
}
