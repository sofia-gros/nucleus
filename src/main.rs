use gpui::*;
use gpui_component::*;
use workspace::Workspace;
use plugin_manager::PluginManager;
use std::path::{Path, PathBuf};
use std::sync::mpsc;
use plugin_manager::PluginManagerGlobal;
use gpui_component::theme::{Theme, ThemeMode};

pub mod editor;
pub mod plugin_manager;
pub mod workspace;
pub mod settings;
pub mod file_system;
pub mod project;
pub mod lsp;
pub mod terminal;
pub mod process;
pub mod keybindings;
pub mod theme;
pub mod search;
pub mod util;

use settings::{SettingsStore, SettingsGlobal};
use std::sync::{Arc, RwLock};

fn main() {
    let (action_tx, action_rx) = mpsc::sync_channel(1024);
    let settings_store = Arc::new(RwLock::new(SettingsStore::new()));
    let settings_for_pm = settings_store.clone();

    let app = gpui_platform::application().with_assets(gpui_component_assets::Assets);

    app.run(move |cx| {
        let settings_store_clone = settings_store.clone();
        cx.set_global(SettingsGlobal(settings_store));

        let pm_model = cx.new(|_cx| {
            let mut pm = PluginManager::new(action_tx.clone(), settings_for_pm).unwrap();
            if let Err(e) = pm.discover_and_load(Path::new("plugins")) {
                eprintln!("Failed to load plugins: {}", e);
            }
            pm
        });
        
        cx.set_global(PluginManagerGlobal(pm_model.clone()));
        // This must be called before using any GPUI Component features.
        gpui_component::init(cx);

        // Sync theme with OS appearance
        let appearance = cx.window_appearance();
        let mode = match appearance {
            WindowAppearance::Dark | WindowAppearance::VibrantDark => ThemeMode::Dark,
            WindowAppearance::Light | WindowAppearance::VibrantLight => ThemeMode::Light,
        };
        Theme::change(mode, None, cx);

        keybindings::init_keybindings(cx);

        let window_cx = cx.to_async();
        cx.spawn(|_cx: &mut gpui::AsyncApp| async move {
            let root_path = window_cx.update(|_cx| {
                // Parse CLI arguments for root path
                let args: Vec<String> = std::env::args().collect();
                let cli_root = if args.len() > 1 {
                    let arg = &args[1];
                    if arg.starts_with("--root=") {
                        Some(PathBuf::from(arg.trim_start_matches("--root=")))
                    } else if !arg.starts_with("--") {
                        Some(PathBuf::from(arg))
                    } else {
                        None
                    }
                } else {
                    None
                };

                if cli_root.is_some() {
                    cli_root
                } else {
                    settings_store_clone.read().unwrap().get("last_opened_workspace").and_then(|v| v.as_str().map(|s| PathBuf::from(s)))
                }
            });
            
            let _ = window_cx.update(|cx| {
                let mut options = WindowOptions::default();
                options.titlebar = Some(gpui::TitlebarOptions {
                    title: None,
                    appears_transparent: true,
                    traffic_light_position: None,
                });
                let _ = cx.open_window(options, move |window, cx| {
                    let view = cx.new(|cx| Workspace::new(root_path, cx));
                    view.update(cx, |workspace, cx| {
                        workspace.focus_handle.focus(window, cx);
                    });
                    
                    let view_clone = view.clone();
                    spawn_plugin_event_loop(cx, action_tx.clone(), action_rx, pm_model, view_clone);

                    // This first level on the window, should be a Root.
                    cx.new(|cx| Root::new(view, window, cx))
                });
            });
        }).detach();
    });
}

fn spawn_plugin_event_loop(
    cx: &mut App,
    action_tx: mpsc::SyncSender<plugin_manager::action::PluginAction>,
    action_rx: mpsc::Receiver<plugin_manager::action::PluginAction>,
    pm_model: gpui::Entity<PluginManager>,
    view_clone: gpui::Entity<Workspace>,
) {
    cx.spawn(async move |app_cx| {
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
                    plugin_manager::action::PluginAction::FileSystemRead { plugin_id: _, req_id, path } => {
                        let tx = action_tx.clone();
                        std::thread::spawn(move || {
                            let result = std::fs::read_to_string(&path);
                            let (content, error) = match result {
                                Ok(c) => (Some(c), None),
                                Err(e) => (None, Some(e.to_string())),
                            };
                            let _ = tx.send(plugin_manager::action::PluginAction::FileSystemReadComplete { req_id, content, error });
                        });
                    }
                    plugin_manager::action::PluginAction::FileSystemWrite { plugin_id: _, req_id, path, content } => {
                        let tx = action_tx.clone();
                        std::thread::spawn(move || {
                            let result = std::fs::write(&path, content);
                            let error = match result {
                                Ok(_) => None,
                                Err(e) => Some(e.to_string()),
                            };
                            let _ = tx.send(plugin_manager::action::PluginAction::FileSystemWriteComplete { req_id, error });
                        });
                    }
                    plugin_manager::action::PluginAction::FileSystemReadComplete { req_id, content, error } => {
                        let _ = app_cx.update(|cx| {
                            pm_model.update(cx, |pm, _| {
                                pm.dispatch_event(plugin_manager::event::PluginEvent::FileSystemReadComplete { req_id, content, error });
                            });
                        });
                    }
                    plugin_manager::action::PluginAction::FileSystemWriteComplete { req_id, error } => {
                        let _ = app_cx.update(|cx| {
                            pm_model.update(cx, |pm, _| {
                                pm.dispatch_event(plugin_manager::event::PluginEvent::FileSystemWriteComplete { req_id, error });
                            });
                        });
                    }
                    plugin_manager::action::PluginAction::RegisterCommand { plugin_id, command } => {
                        let _ = app_cx.update(|cx| {
                            pm_model.update(cx, |pm, _| {
                                pm.register_command(plugin_id, command);
                            });
                        });
                    }
                    plugin_manager::action::PluginAction::ExecuteCommand { command } => {
                        let _ = app_cx.update(|cx| {
                            pm_model.update(cx, |pm, _| {
                                pm.dispatch_event(plugin_manager::event::PluginEvent::CommandExecuted { command });
                            });
                        });
                    }
                    plugin_manager::action::PluginAction::RegisterStatusBarItem { plugin_id, id, text, icon, command, align } => {
                        let _ = app_cx.update(|cx| {
                            pm_model.update(cx, |pm, cx| {
                                let alignment = if align.to_lowercase() == "right" {
                                    plugin_manager::ui::StatusBarAlignment::Right
                                } else {
                                    plugin_manager::ui::StatusBarAlignment::Left
                                };
                                pm.ui_registry.register_status_bar_item(plugin_manager::ui::StatusBarItem {
                                    id, plugin_id, text, icon, command, alignment
                                });
                                cx.notify();
                            });
                        });
                    }
                    plugin_manager::action::PluginAction::RegisterActivityBarItem { plugin_id, id, icon, tooltip, command } => {
                        let _ = app_cx.update(|cx| {
                            pm_model.update(cx, |pm, cx| {
                                pm.ui_registry.register_activity_bar_item(plugin_manager::ui::ActivityBarItem {
                                    id, plugin_id, icon, tooltip, command
                                });
                                cx.notify();
                            });
                        });
                    }
                    plugin_manager::action::PluginAction::RegisterSidebarItem { plugin_id, id, title, ui_ast } => {
                        let _ = app_cx.update(|cx| {
                            pm_model.update(cx, |pm, cx| {
                                pm.ui_registry.register_sidebar_item(plugin_manager::ui::SidebarItem {
                                    id, plugin_id, title, ui_ast
                                });
                                cx.notify();
                            });
                        });
                    }
                    plugin_manager::action::PluginAction::UpdateSidebarItem { plugin_id, id, title, ui_ast } => {
                        let _ = app_cx.update(|cx| {
                            pm_model.update(cx, |pm, cx| {
                                pm.ui_registry.update_sidebar_item(&plugin_id, &id, title, ui_ast);
                                cx.notify();
                            });
                        });
                    }
                    plugin_manager::action::PluginAction::RegisterPanelItem { plugin_id, id, title, ui_ast } => {
                        let _ = app_cx.update(|cx| {
                            pm_model.update(cx, |pm, cx| {
                                pm.ui_registry.register_panel_item(plugin_manager::ui::PanelItem {
                                    id, plugin_id, title, ui_ast
                                });
                                cx.notify();
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
