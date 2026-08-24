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
