/// Material Icon Theme プラグイン実装

use nucleus_plugin_sdk::{export_plugin, log, invoke};

fn init() {
    log("Initializing Material Icon Theme plugin...");

    let icon_rules = serde_json::json!({
        "rules": {
            "rs": { "icon": "🦀", "color": "#f97316" },
            "ts": { "icon": "🔷", "color": "#3b82f6" },
            "tsx": { "icon": "🔷", "color": "#3b82f6" },
            "js": { "icon": "🟨", "color": "#eab308" },
            "jsx": { "icon": "🟨", "color": "#eab308" },
            "json": { "icon": "📋", "color": "#facc15" },
            "toml": { "icon": "⚙️", "color": "#94a3b8" },
            "md": { "icon": "📝", "color": "#38bdf8" },
            "png": { "icon": "🖼️", "color": "#a855f7" },
            "jpg": { "icon": "🖼️", "color": "#a855f7" },
            "svg": { "icon": "🎨", "color": "#ec4899" },
            "go": { "icon": "🐹", "color": "#00add8" },
            "py": { "icon": "🐍", "color": "#306998" },
            "html": { "icon": "🌐", "color": "#e34c26" },
            "css": { "icon": "🎨", "color": "#264de4" },
            "Cargo.toml": { "icon": "📦", "color": "#ea580c" },
            "package.json": { "icon": "📦", "color": "#22c55e" },
            "Dockerfile": { "icon": "🐳", "color": "#0db7ed" },
            ".gitignore": { "icon": "👁️", "color": "#f05032" }
        }
    });

    let args = serde_json::to_string(&icon_rules).unwrap_or_default();
    invoke("ui.register_icon_rules", &args);

    log("Material Icon Theme rules registered successfully.");
}

export_plugin!(init);
