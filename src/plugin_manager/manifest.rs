use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct PluginManifest {
    pub plugin: PluginMetadata,
    #[serde(default)]
    pub runtime: PluginRuntime,
    #[serde(default)]
    pub permissions: PluginPermissions,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct PluginMetadata {
    pub id: String,
    pub name: String,
    pub version: String,
    pub api_version: String,
}

#[derive(Debug, Deserialize, Serialize, Clone, Default)]
pub struct PluginRuntime {
    pub wasm: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, Clone, Default)]
pub struct PluginPermissions {
    #[serde(default)]
    pub filesystem: Vec<String>,
    #[serde(default)]
    pub process: Vec<String>,
    #[serde(default)]
    pub network: bool,
    #[serde(default)]
    pub terminal: bool,
}

impl PluginManifest {
    pub fn parse(toml_content: &str) -> anyhow::Result<Self> {
        let manifest: Self = toml::from_str(toml_content)?;
        Ok(manifest)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_git_plugin_manifest_parsing() {
        let toml_str = r#"
        [plugin]
        id = "nucleus.git"
        name = "Git Source Control"
        version = "0.1.0"
        api_version = "1"

        [runtime]
        wasm = "git_plugin.wasm"

        [permissions]
        filesystem = ["read"]
        process = ["spawn"]
        "#;

        let manifest = PluginManifest::parse(toml_str).unwrap();
        assert_eq!(manifest.plugin.id, "nucleus.git");
        assert_eq!(manifest.plugin.name, "Git Source Control");
        assert_eq!(manifest.runtime.wasm.as_deref(), Some("git_plugin.wasm"));
        assert_eq!(manifest.permissions.filesystem, vec!["read"]);
        assert_eq!(manifest.permissions.process, vec!["spawn"]);
    }
}
