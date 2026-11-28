//! `nucleus-ext`: Extension を公開するための道具。
//!
//! ```text
//! nucleus-ext validate <dir>                 extension.yaml を検証する
//! nucleus-ext pack <dir> [-o <file>]         配布物（.tar.gz）を作り、sha256 を出す
//! nucleus-ext index <registry> [--check]     extensions/*.yaml から index.json を作る（--check は差分があれば失敗）
//! ```

use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
    process::ExitCode,
};

use extension_host::registry::{self, Index, IndexEntry};
use serde::Deserialize;

/// 公開リポジトリの `extensions/<id>.yaml`。
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Listing {
    /// 配布物の場所（リポジトリからの相対パスか URL）。
    download: String,
    sha256: String,
    #[serde(default)]
    repository: Option<String>,
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let result = match args.first().map(String::as_str) {
        Some("validate") => args.get(1).ok_or(usage()).and_then(|d| validate(Path::new(d))),
        Some("pack") => args.get(1).ok_or(usage()).and_then(|d| {
            let out = args
                .iter()
                .position(|a| a == "-o")
                .and_then(|i| args.get(i + 1))
                .map(PathBuf::from);
            pack(Path::new(d), out)
        }),
        Some("index") => args
            .get(1)
            .ok_or(usage())
            .and_then(|d| index(Path::new(d), args.iter().any(|a| a == "--check"))),
        _ => Err(usage()),
    };
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::FAILURE
        }
    }
}

fn usage() -> String {
    "usage: nucleus-ext validate <dir> | pack <dir> [-o <file>] | index <registry> [--check]".into()
}

fn validate(dir: &Path) -> Result<(), String> {
    let m = registry::read_manifest(dir)?;
    if let Some(main) = &m.main
        && !dir.join(main).is_file()
    {
        return Err(format!("main `{main}` がない"));
    }
    println!("ok: {} {} ({})", m.id, m.version, m.name);
    for p in m.permission_list() {
        let mark = if p.strong { " (strong)" } else { "" };
        println!("  permission {}: {}{mark}", p.category, p.value);
    }
    Ok(())
}

fn pack(dir: &Path, out: Option<PathBuf>) -> Result<(), String> {
    validate(dir)?;
    let m = registry::read_manifest(dir)?;
    let bytes = registry::pack(dir)?;
    let out = out.unwrap_or_else(|| PathBuf::from(format!("{}-{}.tar.gz", m.id, m.version)));
    std::fs::write(&out, &bytes).map_err(|e| format!("{}: {e}", out.display()))?;
    println!("wrote {} ({} bytes)", out.display(), bytes.len());
    println!();
    println!("# extensions/{}.yaml", m.id);
    println!(
        "download: <URL or archives/{}>",
        out.file_name().unwrap_or_default().to_string_lossy()
    );
    println!("sha256: {}", registry::sha256_hex(&bytes));
    Ok(())
}

/// 公開リポジトリの一覧を作る。配布物を読んで中身を検証し、ID の重複やファイル名の違いを拒む。
fn index(root: &Path, check: bool) -> Result<(), String> {
    let dir = root.join("extensions");
    let mut files: Vec<PathBuf> = std::fs::read_dir(&dir)
        .map_err(|e| format!("{}: {e}", dir.display()))?
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|x| x == "yaml"))
        .collect();
    files.sort();
    let mut entries: BTreeMap<String, IndexEntry> = BTreeMap::new();
    let mut errors = Vec::new();
    for f in files {
        let name = f.file_stem().unwrap_or_default().to_string_lossy().into_owned();
        match entry(root, &f) {
            Ok(e) if e.id != name => errors.push(format!("{}: 中身の ID は `{}`", f.display(), e.id)),
            Ok(e) => {
                println!("ok: {} {}", e.id, e.version);
                entries.insert(e.id.clone(), e);
            }
            Err(e) => errors.push(format!("{}: {e}", f.display())),
        }
    }
    if !errors.is_empty() {
        return Err(errors.join("\n"));
    }
    let index = Index {
        version: 1,
        extensions: entries.into_values().collect(),
    };
    let json = serde_json::to_string_pretty(&index).map_err(|e| e.to_string())? + "\n";
    let path = root.join("index.json");
    if check {
        let current = std::fs::read_to_string(&path).unwrap_or_default();
        if current != json {
            return Err("index.json が古い。`nucleus-ext index .` で作り直してコミットしてください".into());
        }
        println!("index.json is up to date ({} extensions)", index.extensions.len());
    } else {
        std::fs::write(&path, json).map_err(|e| e.to_string())?;
        println!("wrote {} ({} extensions)", path.display(), index.extensions.len());
    }
    Ok(())
}

fn entry(root: &Path, file: &Path) -> Result<IndexEntry, String> {
    let text = std::fs::read_to_string(file).map_err(|e| e.to_string())?;
    let listing: Listing = serde_yaml_ng::from_str(&text).map_err(|e| e.to_string())?;
    let location = if listing.download.contains("://") {
        listing.download.clone()
    } else {
        if listing.download.split('/').any(|p| p == "..") {
            return Err("download がリポジトリの外を指している".into());
        }
        root.join(&listing.download).display().to_string()
    };
    let bytes = registry::fetch(&location, 50 << 20)?;
    let e = registry::entry_for(&bytes, &listing.download, listing.repository)?;
    if !e.sha256.eq_ignore_ascii_case(&listing.sha256) {
        return Err(format!(
            "sha256 が一致しない（書いてある {}, 実際 {}）",
            listing.sha256, e.sha256
        ));
    }
    Ok(e)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn builds_and_checks_an_index() {
        let d = tempfile::tempdir().unwrap();
        let src = d.path().join("src");
        fs::create_dir_all(&src).unwrap();
        fs::write(
            src.join("extension.yaml"),
            "id: a.b\nname: B\nversion: 0.1.0\npublisher: a\npermissions: { locale: [messages] }\nl10n: l10n\n",
        )
        .unwrap();
        fs::create_dir_all(src.join("l10n")).unwrap();
        fs::write(src.join("l10n/en.ftl"), "a-b-hi = Hi\n").unwrap();
        let reg = d.path().join("reg");
        fs::create_dir_all(reg.join("archives")).unwrap();
        fs::create_dir_all(reg.join("extensions")).unwrap();
        let bytes = registry::pack(&src).unwrap();
        fs::write(reg.join("archives/a.b-0.1.0.tar.gz"), &bytes).unwrap();
        fs::write(
            reg.join("extensions/a.b.yaml"),
            format!(
                "download: archives/a.b-0.1.0.tar.gz\nsha256: {}\n",
                registry::sha256_hex(&bytes)
            ),
        )
        .unwrap();
        assert!(index(&reg, true).is_err(), "no index yet");
        index(&reg, false).unwrap();
        index(&reg, true).unwrap();
        let idx: Index = serde_json::from_str(&fs::read_to_string(reg.join("index.json")).unwrap()).unwrap();
        assert_eq!(idx.extensions[0].id, "a.b");

        // ファイル名と ID が違えば失敗する。
        fs::rename(reg.join("extensions/a.b.yaml"), reg.join("extensions/x.y.yaml")).unwrap();
        assert!(index(&reg, false).unwrap_err().contains("a.b"));
    }
}
