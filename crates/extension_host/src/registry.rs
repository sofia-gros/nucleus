//! Extension の公開リポジトリ（sofia-gros/nucleus-extensions）とのやり取り。バックグラウンドで呼ぶ。
//!
//! - 一覧は `index.json`（リポジトリの CI が各 Extension の配布物から作る）。
//! - 配布物は Extension のフォルダをまとめた `.tar.gz`。sha256 で確かめてから展開し、
//!   中の `extension.yaml` を検証して、一覧と同じ ID・版・権限であるときだけ入れる。
//! - 入れる先は `~/.local/share/nucleus/extensions/<id>/`（ユーザーの Extension と同じ場所）。

use std::{
    collections::BTreeMap,
    io::Read as _,
    path::{Component, Path, PathBuf},
};

use serde::{Deserialize, Serialize};
use sha2::{Digest as _, Sha256};

use crate::manifest::{Api, Manifest};

/// 既定の一覧の場所。設定 `extensions_registry` で変えられる。
pub const DEFAULT_INDEX_URL: &str = "https://raw.githubusercontent.com/sofia-gros/nucleus-extensions/main/index.json";

/// 配布物の大きさの上限。
const MAX_ARCHIVE: u64 = 50 << 20;
/// 展開後の合計の上限。
const MAX_EXTRACTED: u64 = 200 << 20;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Index {
    /// 一覧の形式の版。
    pub version: u32,
    pub extensions: Vec<IndexEntry>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct IndexEntry {
    pub id: String,
    pub name: String,
    pub version: String,
    pub publisher: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub api: Api,
    #[serde(default)]
    pub permissions: BTreeMap<String, Vec<String>>,
    /// 配布物の場所（URL か、一覧からの相対パス）。
    pub download: String,
    pub sha256: String,
    #[serde(default)]
    pub size: u64,
    #[serde(default)]
    pub repository: Option<String>,
}

fn agent() -> ureq::Agent {
    let tls = ureq::tls::TlsConfig::builder()
        .root_certs(ureq::tls::RootCerts::PlatformVerifier)
        .build();
    ureq::Agent::config_builder()
        .tls_config(tls)
        .timeout_global(Some(std::time::Duration::from_secs(60)))
        .user_agent("nucleus-editor")
        .build()
        .new_agent()
}

/// URL かファイルのパスから読む。
pub fn fetch(location: &str, limit: u64) -> Result<Vec<u8>, String> {
    if let Some(path) = location.strip_prefix("file://") {
        return read_file(Path::new(path), limit);
    }
    if !location.starts_with("https://") && !location.starts_with("http://") {
        return read_file(Path::new(location), limit);
    }
    let mut resp = agent().get(location).call().map_err(|e| format!("{location}: {e}"))?;
    resp.body_mut()
        .with_config()
        .limit(limit)
        .read_to_vec()
        .map_err(|e| format!("{location}: {e}"))
}

fn read_file(path: &Path, limit: u64) -> Result<Vec<u8>, String> {
    let meta = std::fs::metadata(path).map_err(|e| format!("{}: {e}", path.display()))?;
    if meta.len() > limit {
        return Err(format!("{}: too large", path.display()));
    }
    std::fs::read(path).map_err(|e| format!("{}: {e}", path.display()))
}

/// 一覧を読む。
pub fn fetch_index(url: &str) -> Result<Index, String> {
    let bytes = fetch(url, 16 << 20)?;
    let index: Index = serde_json::from_slice(&bytes).map_err(|e| format!("index.json: {e}"))?;
    if index.version != 1 {
        return Err(format!("index.json の形式 {} には対応していない", index.version));
    }
    Ok(index)
}

/// 一覧からの相対の場所を解決する。
pub fn resolve(index_url: &str, download: &str) -> String {
    if download.contains("://") || download.starts_with('/') {
        return download.to_string();
    }
    match index_url.rsplit_once('/') {
        Some((base, _)) => format!("{base}/{download}"),
        None => download.to_string(),
    }
}

pub fn sha256_hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes).iter().map(|b| format!("{b:02x}")).collect()
}

/// 一覧の項目を入れる。入れたフォルダを返す。
pub fn install(entry: &IndexEntry, index_url: &str, dest_root: &Path) -> Result<PathBuf, String> {
    let url = resolve(index_url, &entry.download);
    let bytes = fetch(&url, MAX_ARCHIVE)?;
    let sum = sha256_hex(&bytes);
    if !sum.eq_ignore_ascii_case(&entry.sha256) {
        return Err(format!("sha256 が一致しない（期待 {}, 実際 {sum}）", entry.sha256));
    }
    install_archive(&bytes, Some(entry), dest_root)
}

/// 配布物（.tar.gz）を展開して入れる。`entry` があれば ID・版・権限が同じか確かめる。
pub fn install_archive(bytes: &[u8], entry: Option<&IndexEntry>, dest_root: &Path) -> Result<PathBuf, String> {
    std::fs::create_dir_all(dest_root).map_err(|e| e.to_string())?;
    let staging = tempfile_dir(dest_root)?;
    let result = (|| {
        extract(bytes, &staging)?;
        let manifest = read_manifest(&staging)?;
        if let Some(entry) = entry {
            if manifest.id != entry.id || manifest.version != entry.version {
                return Err(format!(
                    "中身（{} {}）が一覧（{} {}）と違う",
                    manifest.id, manifest.version, entry.id, entry.version
                ));
            }
            if manifest.permissions != entry.permissions {
                return Err("中身の権限が一覧と違う".to_string());
            }
        }
        let dest = dest_root.join(&manifest.id);
        if dest.exists() {
            std::fs::remove_dir_all(&dest).map_err(|e| format!("{}: {e}", dest.display()))?;
        }
        std::fs::rename(&staging, &dest).map_err(|e| format!("{}: {e}", dest.display()))?;
        Ok(dest)
    })();
    if result.is_err() {
        let _ = std::fs::remove_dir_all(&staging);
    }
    result
}

/// 入れた Extension を消す（ユーザーのフォルダのものだけ）。
pub fn uninstall(id: &str, dest_root: &Path) -> Result<(), String> {
    if id.contains('/') || id.contains('\\') || id.starts_with('.') || id.is_empty() {
        return Err(format!("ID `{id}` が正しくない"));
    }
    let dir = dest_root.join(id);
    std::fs::remove_dir_all(&dir).map_err(|e| format!("{}: {e}", dir.display()))
}

fn tempfile_dir(root: &Path) -> Result<PathBuf, String> {
    for i in 0..1000u32 {
        let p = root.join(format!(".installing-{}-{i}", std::process::id()));
        if std::fs::create_dir(&p).is_ok() {
            return Ok(p);
        }
    }
    Err("作業用のフォルダを作れない".into())
}

/// フォルダの中の extension.yaml を読んで検証する。
pub fn read_manifest(dir: &Path) -> Result<Manifest, String> {
    let text = std::fs::read_to_string(dir.join("extension.yaml")).map_err(|e| format!("extension.yaml: {e}"))?;
    let m = Manifest::from_yaml(&text).map_err(|e| format!("extension.yaml: {e}"))?;
    let errors = m.validate();
    if !errors.is_empty() {
        return Err(errors.join(" / "));
    }
    Ok(m)
}

/// 展開する。パスの外への脱出・リンク・特殊なファイルは拒む。
fn extract(bytes: &[u8], dest: &Path) -> Result<(), String> {
    let gz = flate2::read::GzDecoder::new(bytes);
    let mut archive = tar::Archive::new(gz);
    let mut total = 0u64;
    for entry in archive.entries().map_err(|e| e.to_string())? {
        let mut entry = entry.map_err(|e| e.to_string())?;
        let path = entry.path().map_err(|e| e.to_string())?.into_owned();
        if path
            .components()
            .any(|c| !matches!(c, Component::Normal(_) | Component::CurDir))
        {
            return Err(format!("`{}` は外を指している", path.display()));
        }
        let kind = entry.header().entry_type();
        let target = dest.join(&path);
        if kind.is_dir() {
            std::fs::create_dir_all(&target).map_err(|e| e.to_string())?;
            continue;
        }
        if !kind.is_file() {
            return Err(format!("`{}` はふつうのファイルではない", path.display()));
        }
        total += entry.size();
        if total > MAX_EXTRACTED {
            return Err("展開すると大きすぎる".into());
        }
        if let Some(parent) = target.parent() {
            std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        let mut data = Vec::new();
        entry.read_to_end(&mut data).map_err(|e| e.to_string())?;
        std::fs::write(&target, data).map_err(|e| e.to_string())?;
    }
    Ok(())
}

/// Extension のフォルダを配布物（.tar.gz）にする。中身の順と時刻は固定（同じ中身なら同じ sha256）。
pub fn pack(dir: &Path) -> Result<Vec<u8>, String> {
    read_manifest(dir)?;
    let mut files = Vec::new();
    collect(dir, dir, &mut files)?;
    files.sort();
    let gz = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::best());
    let mut tar = tar::Builder::new(gz);
    tar.mode(tar::HeaderMode::Deterministic);
    for rel in files {
        let data = std::fs::read(dir.join(&rel)).map_err(|e| e.to_string())?;
        let mut header = tar::Header::new_gnu();
        header.set_size(data.len() as u64);
        header.set_mode(0o644);
        header.set_mtime(0);
        header.set_cksum();
        tar.append_data(&mut header, &rel, data.as_slice())
            .map_err(|e| e.to_string())?;
    }
    tar.into_inner().and_then(|gz| gz.finish()).map_err(|e| e.to_string())
}

fn collect(root: &Path, dir: &Path, out: &mut Vec<String>) -> Result<(), String> {
    for e in std::fs::read_dir(dir).map_err(|e| e.to_string())? {
        let e = e.map_err(|e| e.to_string())?;
        let name = e.file_name().to_string_lossy().into_owned();
        // 隠しファイルとビルドの残りは入れない。
        if name.starts_with('.') || name == "target" {
            continue;
        }
        let path = e.path();
        let ty = e.file_type().map_err(|e| e.to_string())?;
        if ty.is_dir() {
            collect(root, &path, out)?;
        } else if ty.is_file() {
            let rel = path.strip_prefix(root).unwrap_or(&path);
            out.push(rel.to_string_lossy().replace('\\', "/"));
        }
    }
    Ok(())
}

/// 配布物から一覧の項目を作る（公開用の CLI と CI が使う）。
pub fn entry_for(archive: &[u8], download: &str, repository: Option<String>) -> Result<IndexEntry, String> {
    let tmp = std::env::temp_dir().join(format!("nucleus-entry-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&tmp);
    std::fs::create_dir_all(&tmp).map_err(|e| e.to_string())?;
    let r = extract(archive, &tmp).and_then(|_| read_manifest(&tmp));
    let _ = std::fs::remove_dir_all(&tmp);
    let m = r?;
    Ok(IndexEntry {
        id: m.id,
        name: m.name,
        version: m.version,
        publisher: m.publisher,
        description: m.description,
        api: m.api,
        permissions: m.permissions,
        download: download.to_string(),
        sha256: sha256_hex(archive),
        size: archive.len() as u64,
        repository,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn sample(dir: &Path) {
        fs::create_dir_all(dir.join("themes")).unwrap();
        fs::write(
            dir.join("extension.yaml"),
            "id: someone.calm\nname: Calm\nversion: 1.2.0\npublisher: someone\npermissions: { theme: [colors] }\nthemes: [themes/calm.json]\n",
        )
        .unwrap();
        fs::write(dir.join("themes/calm.json"), "{}").unwrap();
        fs::write(dir.join(".DS_Store"), "x").unwrap();
    }

    #[test]
    fn pack_install_and_uninstall() {
        let d = tempfile::tempdir().unwrap();
        let src = d.path().join("src");
        sample(&src);
        let a = pack(&src).unwrap();
        assert_eq!(a, pack(&src).unwrap(), "pack is deterministic");

        let entry = entry_for(&a, "archives/calm-1.2.0.tar.gz", None).unwrap();
        assert_eq!(entry.id, "someone.calm");
        assert_eq!(entry.sha256, sha256_hex(&a));

        // 一覧と配布物を置いて、file:// から入れる。
        let reg = d.path().join("registry");
        fs::create_dir_all(reg.join("archives")).unwrap();
        fs::write(reg.join("archives/calm-1.2.0.tar.gz"), &a).unwrap();
        let index = Index {
            version: 1,
            extensions: vec![entry.clone()],
        };
        fs::write(reg.join("index.json"), serde_json::to_vec(&index).unwrap()).unwrap();
        let url = format!("file://{}", reg.join("index.json").display());
        let fetched = fetch_index(&url).unwrap();
        assert_eq!(fetched, index);

        let dest = d.path().join("installed");
        let dir = install(&fetched.extensions[0], &url, &dest).unwrap();
        assert_eq!(dir, dest.join("someone.calm"));
        assert!(dir.join("themes/calm.json").is_file());
        assert!(!dir.join(".DS_Store").exists());
        // 作業用のフォルダは残らない。
        assert_eq!(fs::read_dir(&dest).unwrap().count(), 1);

        uninstall("someone.calm", &dest).unwrap();
        assert!(!dir.exists());
        assert!(uninstall("../x", &dest).is_err());
    }

    #[test]
    fn tampered_archives_are_rejected() {
        let d = tempfile::tempdir().unwrap();
        let src = d.path().join("src");
        sample(&src);
        let a = pack(&src).unwrap();
        let mut entry = entry_for(&a, "a.tar.gz", None).unwrap();
        fs::write(d.path().join("a.tar.gz"), &a).unwrap();
        let url = format!("{}/index.json", d.path().display());
        let dest = d.path().join("installed");

        let mut wrong_sum = entry.clone();
        wrong_sum.sha256 = "00".repeat(32);
        assert!(install(&wrong_sum, &url, &dest).unwrap_err().contains("sha256"));

        // 一覧の権限と中身が違う（一覧を書き換えた）なら入れない。
        entry.permissions.clear();
        assert!(install(&entry, &url, &dest).unwrap_err().contains("権限"));
        assert!(!dest.join("someone.calm").exists());
    }

    #[test]
    fn archives_cannot_escape() {
        let d = tempfile::tempdir().unwrap();
        let gz = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::fast());
        let mut tar = tar::Builder::new(gz);
        let mut h = tar::Header::new_gnu();
        let data = b"x";
        h.set_size(1);
        h.set_mode(0o644);
        // append_data はパスを確かめるので、名前を直接書く。
        h.as_gnu_mut().unwrap().name[..9].copy_from_slice(b"../evil.t");
        h.set_cksum();
        tar.append(&h, &data[..]).unwrap();
        let bytes = tar.into_inner().unwrap().finish().unwrap();
        let err = install_archive(&bytes, None, &d.path().join("dest")).unwrap_err();
        assert!(err.contains("外"), "{err}");
        assert!(!d.path().join("evil.t").exists());
    }

    #[test]
    fn relative_downloads_resolve_against_the_index() {
        assert_eq!(
            resolve("https://x.test/repo/main/index.json", "archives/a.tar.gz"),
            "https://x.test/repo/main/archives/a.tar.gz"
        );
        assert_eq!(
            resolve("https://x/index.json", "https://y/a.tar.gz"),
            "https://y/a.tar.gz"
        );
    }
}
