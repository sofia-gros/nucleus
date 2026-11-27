//! Extension を探して読み込む（バックグラウンドで呼ぶ）。組み込みはアプリに埋め込み、ユーザーのものはフォルダから読む。

use std::{
    borrow::Cow,
    path::{Path, PathBuf},
};

use crate::manifest::Manifest;

/// アプリに埋め込む組み込み Extension（リポジトリの `extensions/`）。
#[derive(rust_embed::RustEmbed)]
#[folder = "../../extensions"]
struct Builtin;

/// Extension のファイルの読み先。
pub enum Source {
    Builtin(String),
    Dir(PathBuf),
}

impl Source {
    fn read(&self, rel: &str) -> Result<Cow<'static, [u8]>, String> {
        let rel = rel.trim_start_matches("./");
        if rel.split('/').any(|p| p == "..") {
            return Err(format!("`{rel}` は Extension の外を指している"));
        }
        match self {
            Source::Builtin(dir) => Builtin::get(&format!("{dir}/{rel}"))
                .map(|f| f.data)
                .ok_or_else(|| format!("`{rel}` がない")),
            Source::Dir(dir) => std::fs::read(dir.join(rel))
                .map(Cow::Owned)
                .map_err(|e| format!("`{rel}`: {e}")),
        }
    }

    /// `dir` 直下のファイル名。
    fn list(&self, dir: &str) -> Vec<String> {
        let dir = dir.trim_end_matches('/').trim_start_matches("./");
        match self {
            Source::Builtin(root) => {
                let prefix = format!("{root}/{dir}/");
                Builtin::iter()
                    .filter_map(|p| p.strip_prefix(&prefix).map(str::to_string))
                    .filter(|p| !p.contains('/'))
                    .collect()
            }
            Source::Dir(root) => std::fs::read_dir(root.join(dir))
                .map(|it| {
                    it.flatten()
                        .map(|e| e.file_name().to_string_lossy().into_owned())
                        .collect()
                })
                .unwrap_or_default(),
        }
    }
}

/// 読み込んだ Extension（データ系の中身まで読んだもの）。
#[derive(Debug)]
pub struct LoadedExtension {
    pub manifest: Manifest,
    pub builtin: bool,
    pub dir: Option<PathBuf>,
    pub themes: Vec<theme::ThemeFile>,
    pub messages: Vec<i18n::Messages>,
    /// 読み込めなかった部分（Extension 自体は有効なまま）。
    pub warnings: Vec<String>,
}

/// 読み込みに失敗した Extension。
#[derive(Debug)]
pub struct FailedExtension {
    pub location: String,
    pub errors: Vec<String>,
}

pub fn user_extensions_dir() -> Option<PathBuf> {
    dirs::data_dir().map(|d| d.join("nucleus").join("extensions"))
}

/// すべての Extension を探して読む。同じ ID ならユーザーのものを優先する。
pub fn discover(user_dir: Option<&Path>) -> (Vec<LoadedExtension>, Vec<FailedExtension>) {
    let mut loaded: Vec<LoadedExtension> = Vec::new();
    let mut failed = Vec::new();
    let mut builtin_dirs: Vec<String> = Builtin::iter()
        .filter_map(|p| p.strip_suffix("/extension.yaml").map(str::to_string))
        .filter(|p| !p.contains('/'))
        .collect();
    builtin_dirs.sort();
    for dir in builtin_dirs {
        match load(Source::Builtin(dir.clone())) {
            Ok(e) => loaded.push(e),
            Err(errors) => failed.push(FailedExtension {
                location: format!("builtin:{dir}"),
                errors,
            }),
        }
    }
    if let Some(user_dir) = user_dir
        && let Ok(entries) = std::fs::read_dir(user_dir)
    {
        let mut dirs: Vec<PathBuf> = entries
            .flatten()
            .map(|e| e.path())
            .filter(|p| p.join("extension.yaml").is_file())
            .collect();
        dirs.sort();
        for dir in dirs {
            match load(Source::Dir(dir.clone())) {
                Ok(e) => {
                    if let Some(pos) = loaded.iter().position(|l| l.manifest.id == e.manifest.id) {
                        log::info!("Extension `{}` はユーザーのものを使う", e.manifest.id);
                        loaded.remove(pos);
                    }
                    loaded.push(e);
                }
                Err(errors) => failed.push(FailedExtension {
                    location: dir.display().to_string(),
                    errors,
                }),
            }
        }
    }
    (loaded, failed)
}

pub fn load(source: Source) -> Result<LoadedExtension, Vec<String>> {
    let bytes = source.read("extension.yaml").map_err(|e| vec![e])?;
    let text =
        String::from_utf8(bytes.into_owned()).map_err(|_| vec!["extension.yaml が UTF-8 ではない".to_string()])?;
    let manifest = Manifest::from_yaml(&text).map_err(|e| vec![format!("extension.yaml: {e}")])?;
    let errors = manifest.validate();
    if !errors.is_empty() {
        return Err(errors);
    }
    let mut warnings = Vec::new();

    let mut themes = Vec::new();
    for rel in &manifest.themes {
        match source.read(rel).and_then(|b| {
            theme::ThemeFile::from_json(&String::from_utf8_lossy(&b)).map_err(|e| format!("`{rel}`: {e}"))
        }) {
            Ok(t) => themes.push(t),
            Err(e) => warnings.push(e),
        }
    }

    let mut messages = Vec::new();
    if let Some(dir) = &manifest.l10n {
        let pack = manifest.locale.as_ref();
        for file in source.list(dir) {
            let Some(stem) = file.strip_suffix(".ftl") else {
                continue;
            };
            // 言語パックは `l10n/<対象>.ftl`（言語は locale.language）。自分の文言は `l10n/<言語>.ftl`。
            let (lang, kind) = match pack {
                Some(p) => (p.language.as_str(), i18n::SourceKind::Pack),
                None => (stem, i18n::SourceKind::Own),
            };
            let Ok(lang) = lang.parse() else {
                warnings.push(format!("`{file}`: 言語タグ `{lang}` が読めない"));
                continue;
            };
            match source.read(&format!("{dir}/{file}")) {
                Ok(b) => messages.push(i18n::Messages {
                    lang,
                    origin: manifest.id.clone(),
                    kind,
                    ftl: String::from_utf8_lossy(&b).into_owned(),
                }),
                Err(e) => warnings.push(e),
            }
        }
    }

    let (builtin, dir) = match source {
        Source::Builtin(_) => (true, None),
        Source::Dir(d) => (false, Some(d)),
    };
    Ok(LoadedExtension {
        manifest,
        builtin,
        dir,
        themes,
        messages,
        warnings,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn builtin_extensions_load() {
        let (loaded, failed) = discover(None);
        assert!(failed.is_empty(), "{failed:?}");
        let nord = loaded
            .iter()
            .find(|e| e.manifest.id == "nucleus.theme-nord")
            .expect("nord is bundled");
        assert!(nord.builtin);
        assert_eq!(nord.themes[0].name, "Nord");
        assert!(nord.warnings.is_empty(), "{:?}", nord.warnings);
    }

    #[test]
    fn user_language_packs_and_overrides_load_from_disk() {
        let d = tempfile::tempdir().unwrap();
        let ext = d.path().join("es");
        fs::create_dir_all(ext.join("l10n")).unwrap();
        fs::write(
            ext.join("extension.yaml"),
            "id: someone.locale-es\nname: Español\nversion: 0.1.0\npublisher: someone\npermissions: { locale: [pack] }\nl10n: l10n\nlocale: { language: es, label: Español, translates: [core] }\n",
        )
        .unwrap();
        fs::write(ext.join("l10n/core.ftl"), "core-menu-file = Archivo\n").unwrap();
        let broken = d.path().join("broken");
        fs::create_dir_all(&broken).unwrap();
        fs::write(broken.join("extension.yaml"), "id: Bad\n").unwrap();

        let (loaded, failed) = discover(Some(d.path()));
        let es = loaded.iter().find(|e| e.manifest.id == "someone.locale-es").unwrap();
        assert_eq!(es.messages.len(), 1);
        assert_eq!(es.messages[0].lang.to_string(), "es");
        assert_eq!(es.messages[0].kind, i18n::SourceKind::Pack);
        assert_eq!(failed.len(), 1);
    }

    #[test]
    fn paths_cannot_escape_the_extension() {
        let d = tempfile::tempdir().unwrap();
        fs::write(
            d.path().join("extension.yaml"),
            "id: a.b\nname: B\nversion: 1.0.0\npublisher: a\npermissions: { theme: [colors] }\nthemes: [../../etc/passwd]\n",
        )
        .unwrap();
        let e = load(Source::Dir(d.path().to_path_buf())).unwrap();
        assert!(e.themes.is_empty());
        assert!(e.warnings[0].contains("外"));
    }
}
