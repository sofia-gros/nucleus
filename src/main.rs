use gpui::*;
use gpui_component::*;
use workspace::Workspace;
use plugin_manager::PluginManager;
use std::path::Path;
use std::sync::mpsc;
use plugin_manager::PluginManagerGlobal;

pub mod editor;
pub mod plugin_manager;
pub mod workspace;
pub mod settings;

use settings::{SettingsStore, SettingsGlobal};
use std::sync::{Arc, RwLock};

fn main() {
    let (action_tx, action_rx) = mpsc::sync_channel(1024);
    let settings_store = Arc::new(RwLock::new(SettingsStore::new()));
    let settings_for_pm = settings_store.clone();

    let app = gpui_platform::application().with_assets(gpui_component_assets::Assets);

    app.run(move |cx| {
        cx.set_global(SettingsGlobal(settings_store));

        let pm_model = cx.new(|_cx| {
            let mut pm = PluginManager::new(action_tx, settings_for_pm).unwrap();
            if let Err(e) = pm.discover_and_load(Path::new("plugins")) {
                eprintln!("Failed to load plugins: {}", e);
            }
            pm
        });
        
        cx.set_global(PluginManagerGlobal(pm_model.clone()));
        // This must be called before using any GPUI Component features.
        gpui_component::init(cx);

        cx.bind_keys([
            KeyBinding::new("ctrl-b", workspace::ToggleLeftSidebar, None),
            KeyBinding::new("ctrl-j", workspace::ToggleBottomPanel, None),
            KeyBinding::new("ctrl-r", workspace::ToggleRightSidebar, None),
        ]);

        cx.spawn(async move |app_cx| {
            let _ = app_cx.update(|cx| {
                let _ = cx.open_window(WindowOptions::default(), |window, cx| {
                    let view = cx.new(|cx| Workspace::new(cx));
                    view.update(cx, |workspace, _cx| {
                        workspace.focus_handle.focus(window, _cx);
                    });
                    
                    let view_clone = view.clone();
                    spawn_plugin_event_loop(cx, action_rx, pm_model, view_clone);

                    // This first level on the window, should be a Root.
                    cx.new(|cx| Root::new(view, window, cx))
                });
            });
        }).detach();
    });
}

fn spawn_plugin_event_loop(
    cx: &mut App,
    action_rx: mpsc::Receiver<plugin_manager::action::PluginAction>,
    pm_model: gpui::Entity<PluginManager>,
    view_clone: gpui::Entity<Workspace>,
) {
    cx.spawn(async move |mut app_cx| {
        loop {
            while let Ok(action) = action_rx.try_recv() {
                match action {
                    plugin_manager::action::PluginAction::InternalProcessOutput { id, stdout, code } => {
                        let _ = app_cx.update(|cx| {
                            pm_model.update(cx, |pm, _| {
                                pm.dispatch_event(plugin_manager::event::PluginEvent::ProcessOutput { id: id.clone(), stdout });
                                pm.dispatch_event(plugin_manager::event::PluginEvent::ProcessExited { id, code });
                            });
                        });
                    }
                    _ => {
                        let _ = app_cx.update(|cx| {
                            view_clone.update(cx, |workspace, cx| {
                                workspace.handle_action(action, cx);
                            });
                        });
                    }
                }
            }
            app_cx.background_executor().timer(std::time::Duration::from_millis(16)).await;
        }
    }).detach();
}
