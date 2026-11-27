//! 多言語化（docs/ARCHITECTURE.md §6）。UI の文言はすべてここを通る。
//!
//! - 形式は Fluent。コードでは `core.menu.file` のように `.` 区切りで書き、
//!   Fluent のキーとしては `core-menu-file` で引く（Fluent の識別子に `.` は使えないため）。
//! - 訳を探す順番: 選んだ言語の言語パック → 同じ言語の Extension 自身の訳 → 英語。

use std::{
    cell::RefCell,
    collections::{BTreeSet, HashMap},
};

use fluent_bundle::{FluentArgs, FluentBundle, FluentResource, FluentValue};
use gpui_kit::{App, Global, SharedString};
pub use unic_langid::LanguageIdentifier;

/// 文言の出どころの種類。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourceKind {
    /// Core や Extension 自身が持つ訳（`locale.messages`）。
    Own,
    /// 言語パック（`locale.pack`）。同じ言語では Own より優先する。
    Pack,
}

/// 1つの `.ftl` ファイル。
#[derive(Clone, Debug)]
pub struct Messages {
    pub lang: LanguageIdentifier,
    /// `core` または Extension の ID。
    pub origin: String,
    pub kind: SourceKind,
    pub ftl: String,
}

const CORE_EN: &str = include_str!("../locales/en/core.ftl");
const CORE_JA: &str = include_str!("../locales/ja/core.ftl");

pub fn en() -> LanguageIdentifier {
    "en".parse().expect("valid tag")
}

/// 文言ファイルの集まり。ここから言語ごとの [`Localizer`] を作る。
#[derive(Clone, Debug, Default)]
pub struct Catalog {
    sources: Vec<Messages>,
}

impl Catalog {
    /// Core の英語と日本語を含むカタログ。
    pub fn with_core() -> Self {
        let mut c = Catalog::default();
        for (lang, ftl) in [("en", CORE_EN), ("ja", CORE_JA)] {
            c.add(Messages {
                lang: lang.parse().expect("valid tag"),
                origin: "core".into(),
                kind: SourceKind::Own,
                ftl: ftl.into(),
            })
            .expect("bundled ftl is valid");
        }
        c
    }

    /// 文言ファイルを追加する。構文エラーがあれば追加しない。
    pub fn add(&mut self, m: Messages) -> Result<(), Vec<String>> {
        if let Err((_, errors)) = FluentResource::try_new(m.ftl.clone()) {
            return Err(errors
                .iter()
                .map(|e| format!("{} ({}): {e:?}", m.origin, m.lang))
                .collect());
        }
        self.sources.push(m);
        Ok(())
    }

    /// 選べる言語（何かしらの訳があるもの）。
    pub fn languages(&self) -> Vec<LanguageIdentifier> {
        let set: BTreeSet<String> = self.sources.iter().map(|s| s.lang.to_string()).collect();
        set.into_iter().filter_map(|s| s.parse().ok()).collect()
    }

    /// 探す言語の順番: `ja-JP` → `ja` → `en`。
    fn candidates(requested: &LanguageIdentifier) -> Vec<LanguageIdentifier> {
        let mut out = vec![requested.clone()];
        let base: LanguageIdentifier = requested.language.as_str().parse().unwrap_or_else(|_| en());
        if base != *requested {
            out.push(base);
        }
        if !out.iter().any(|l| *l == en()) {
            out.push(en());
        }
        out
    }

    pub fn localizer(&self, requested: &LanguageIdentifier) -> Localizer {
        let mut chain = Vec::new();
        for lang in Self::candidates(requested) {
            let mut bundle = FluentBundle::new(vec![lang.clone()]);
            bundle.set_use_isolating(false);
            let mut any = false;
            // Own を先に入れ、Pack で上書きする（同じ言語では言語パックが優先）。
            for kind in [SourceKind::Own, SourceKind::Pack] {
                for s in self.sources.iter().filter(|s| s.lang == lang && s.kind == kind) {
                    let res = FluentResource::try_new(s.ftl.clone()).expect("validated in add()");
                    bundle.add_resource_overriding(res);
                    any = true;
                }
            }
            if any {
                chain.push(bundle);
            }
        }
        Localizer {
            lang: requested.clone(),
            chain,
            memo: RefCell::new(HashMap::new()),
        }
    }

    /// 翻訳の進み具合: 英語にあるキーのうち、`lang` で訳されている数。
    pub fn coverage(&self, lang: &LanguageIdentifier) -> (usize, usize) {
        let keys = |l: &LanguageIdentifier| -> BTreeSet<String> {
            self.sources
                .iter()
                .filter(|s| s.lang == *l)
                .filter_map(|s| FluentResource::try_new(s.ftl.clone()).ok())
                .flat_map(|r| {
                    r.entries()
                        .filter_map(|e| match e {
                            fluent_syntax::ast::Entry::Message(m) => Some(m.id.name.to_string()),
                            _ => None,
                        })
                        .collect::<Vec<_>>()
                })
                .collect()
        };
        let base = keys(&en());
        let done = keys(lang).intersection(&base).count();
        (done, base.len())
    }
}

/// Fluent のキーへの変換（`.` → `-`）。
pub fn fluent_id(key: &str) -> String {
    key.replace('.', "-")
}

/// 1つの言語用に組み立てた文言の表。
pub struct Localizer {
    lang: LanguageIdentifier,
    chain: Vec<FluentBundle<FluentResource>>,
    memo: RefCell<HashMap<String, SharedString>>,
}

impl Localizer {
    pub fn language(&self) -> &LanguageIdentifier {
        &self.lang
    }

    /// 引数なしの文言。結果は覚えておく（描画のたびに組み立て直さない）。
    pub fn t(&self, key: &str) -> SharedString {
        if let Some(s) = self.memo.borrow().get(key) {
            return s.clone();
        }
        let s: SharedString = self.format(key, None).into();
        self.memo.borrow_mut().insert(key.to_string(), s.clone());
        s
    }

    /// 引数つきの文言（複数形や変数を含むもの）。
    pub fn t_args(&self, key: &str, args: &[(&str, FluentValue<'_>)]) -> SharedString {
        let mut fa = FluentArgs::new();
        for (k, v) in args {
            fa.set(*k, v.clone());
        }
        self.format(key, Some(&fa)).into()
    }

    fn format(&self, key: &str, args: Option<&FluentArgs>) -> String {
        let id = fluent_id(key);
        for bundle in &self.chain {
            let Some(msg) = bundle.get_message(&id) else { continue };
            let Some(pattern) = msg.value() else { continue };
            let mut errors = Vec::new();
            let s = bundle.format_pattern(pattern, args, &mut errors);
            if !errors.is_empty() {
                log::warn!("i18n `{key}`: {errors:?}");
            }
            return s.into_owned();
        }
        log::warn!("i18n: `{key}` の訳がない（{}）", self.lang);
        key.to_string()
    }
}

/// gpui の Global。
pub struct I18n {
    pub catalog: Catalog,
    pub localizer: Localizer,
}

impl Global for I18n {}

/// 文言を取り出す。
pub trait Translate {
    fn t(&self, key: &str) -> SharedString;
    fn t_args(&self, key: &str, args: &[(&str, FluentValue<'_>)]) -> SharedString;
}

impl Translate for App {
    fn t(&self, key: &str) -> SharedString {
        self.global::<I18n>().localizer.t(key)
    }

    fn t_args(&self, key: &str, args: &[(&str, FluentValue<'_>)]) -> SharedString {
        self.global::<I18n>().localizer.t_args(key, args)
    }
}

/// 環境変数から言語を決める（`NUCLEUS_LANG` → `LC_ALL` → `LC_MESSAGES` → `LANG`）。
pub fn detect_language() -> LanguageIdentifier {
    for var in ["NUCLEUS_LANG", "LC_ALL", "LC_MESSAGES", "LANG"] {
        if let Ok(v) = std::env::var(var) {
            // `ja_JP.UTF-8` → `ja-JP`
            let tag = v.split(['.', '@']).next().unwrap_or("").replace('_', "-");
            if tag.is_empty() || tag == "C" || tag == "POSIX" {
                continue;
            }
            if let Ok(l) = tag.parse() {
                return l;
            }
        }
    }
    en()
}

pub fn init(lang: LanguageIdentifier, cx: &mut App) {
    let catalog = Catalog::with_core();
    let localizer = catalog.localizer(&lang);
    cx.set_global(I18n { catalog, localizer });
}

/// 言語を切り替える。すべてのウィンドウを描き直す。
pub fn set_language(lang: LanguageIdentifier, cx: &mut App) {
    let localizer = cx.global::<I18n>().catalog.localizer(&lang);
    cx.global_mut::<I18n>().localizer = localizer;
    cx.refresh_windows();
}

/// 文言ファイルを追加し、今の言語の表を作り直す。
pub fn add_messages(m: Messages, cx: &mut App) -> Result<(), Vec<String>> {
    let lang = cx.global::<I18n>().localizer.language().clone();
    let i18n = cx.global_mut::<I18n>();
    i18n.catalog.add(m)?;
    i18n.localizer = i18n.catalog.localizer(&lang);
    cx.refresh_windows();
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tag(s: &str) -> LanguageIdentifier {
        s.parse().unwrap()
    }

    #[test]
    fn falls_back_from_region_to_language_to_english() {
        let c = Catalog::with_core();
        assert_eq!(c.localizer(&tag("ja-JP")).t("core.menu.file"), "ファイル");
        assert_eq!(c.localizer(&tag("fr")).t("core.menu.file"), "File");
    }

    #[test]
    fn missing_keys_show_the_key() {
        let c = Catalog::with_core();
        assert_eq!(c.localizer(&tag("en")).t("core.nope"), "core.nope");
    }

    #[test]
    fn plurals_and_variables() {
        let l = Catalog::with_core().localizer(&tag("en"));
        assert_eq!(
            l.t_args("core.status.extensions-loading", &[("count", 1.into())]),
            "Loading 1 extension…"
        );
        assert_eq!(
            l.t_args("core.status.extensions-loading", &[("count", 3.into())]),
            "Loading 3 extensions…"
        );
        assert_eq!(
            l.t_args("core.status.cursor", &[("line", 42.into()), ("column", 17.into())]),
            "42:17"
        );
    }

    #[test]
    fn language_packs_override_own_messages_and_extensions_fall_back_to_english() {
        let mut c = Catalog::with_core();
        c.add(Messages {
            lang: tag("en"),
            origin: "anthropic.claude".into(),
            kind: SourceKind::Own,
            ftl: "anthropic-claude-ask = Ask Claude\nanthropic-claude-name = Claude".into(),
        })
        .unwrap();
        c.add(Messages {
            lang: tag("ja"),
            origin: "anthropic.claude".into(),
            kind: SourceKind::Own,
            ftl: "anthropic-claude-ask = Claude に聞く".into(),
        })
        .unwrap();
        c.add(Messages {
            lang: tag("ja"),
            origin: "nucleus.locale-ja".into(),
            kind: SourceKind::Pack,
            ftl: "anthropic-claude-ask = Claude に質問".into(),
        })
        .unwrap();
        // 新しい言語を言語パックで追加
        c.add(Messages {
            lang: tag("es"),
            origin: "nucleus.locale-es".into(),
            kind: SourceKind::Pack,
            ftl: "core-menu-file = Archivo".into(),
        })
        .unwrap();

        let ja = c.localizer(&tag("ja"));
        assert_eq!(ja.t("anthropic.claude.ask"), "Claude に質問");
        assert_eq!(ja.t("anthropic.claude.name"), "Claude");
        let es = c.localizer(&tag("es-MX"));
        assert_eq!(es.t("core.menu.file"), "Archivo");
        assert_eq!(es.t("core.menu.edit"), "Edit");
        assert!(c.languages().contains(&tag("es")));
    }

    #[test]
    fn rejects_broken_ftl() {
        let mut c = Catalog::default();
        assert!(
            c.add(Messages {
                lang: en(),
                origin: "x".into(),
                kind: SourceKind::Own,
                ftl: "= broken".into()
            })
            .is_err()
        );
    }

    #[test]
    fn japanese_covers_every_core_key() {
        let c = Catalog::with_core();
        let (done, total) = c.coverage(&tag("ja"));
        assert_eq!(done, total, "ja is missing core keys");
    }

    #[test]
    fn detects_posix_locale_names() {
        // SAFETY: テスト内でだけ環境変数を書き換える（このテストは並列の他テストと変数を共有しない）。
        unsafe { std::env::set_var("NUCLEUS_LANG", "ja_JP.UTF-8") };
        assert_eq!(detect_language(), tag("ja-JP"));
        unsafe { std::env::remove_var("NUCLEUS_LANG") };
    }
}
