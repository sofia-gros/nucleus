use gpui::*;
use gpui_component::*;
use workspace::Workspace;
use plugin_manager::PluginManager;
use std::path::Path;
use std::sync::mpsc;
use plugin_manager::PluginManagerGlobal;

pub mod editor;
pub mod plugin_manager;
mod workspace;

fn main() {
    let (action_tx, action_rx) = mpsc::sync_channel(1024);

    let app = gpui_platform::application().with_assets(gpui_component_assets::Assets);

    app.run(move |cx| {
        let pm_model = cx.new(|_cx| {
            let mut pm = PluginManager::new(action_tx).unwrap();
            if let Err(e) = pm.discover_and_load(Path::new("plugins")) {
                eprintln!("Failed to load plugins: {}", e);
            }
            pm
        });
        
        cx.set_global(PluginManagerGlobal(pm_model));
        // This must be called before using any GPUI Component features.
        gpui_component::init(cx);

        cx.spawn(async move |cx| {
            cx.open_window(WindowOptions::default(), move |window, cx| {
                let view = cx.new(|cx| Workspace::new(cx));
                
                let view_clone = view.clone();
                cx.spawn(async move |mut app_cx| {
                    loop {
                        while let Ok(action) = action_rx.try_recv() {
                            let _ = app_cx.update(|cx| {
                                view_clone.update(cx, |workspace, cx| {
                                    workspace.handle_action(action, cx);
                                });
                            });
                        }
                        app_cx.background_executor().timer(std::time::Duration::from_millis(16)).await;
                    }
                }).detach();

                // This first level on the window, should be a Root.
                cx.new(|cx| Root::new(view, window, cx))
            })
            .expect("Failed to open window");
        })
        .detach();
    });
}
