//! アプリの素材。gpui-kit 既定のアイコンに、Nucleus が使う Lucide アイコンを足す。

use std::borrow::Cow;

use gpui_kit::{AssetSource, Result, SharedString, assets::Assets as KitAssets, assets::icon_assets};

icon_assets!(pub NucleusIcons, [
    Atom, ListTree, GitBranch, Terminal, CircleX, Bell, Columns2, PanelLeft, PanelRight,
    FileCode, Folder, FolderOpen, Command, Keyboard, Settings2, Package, Search,
    TriangleAlert, ScrollText, GitPullRequest, Check, RefreshCw, Save, X, Palette, Languages, LogOut, Info,
]);

/// gpui-kit 既定の素材 → Nucleus のアイコン の順に探す。
pub struct Assets;

impl AssetSource for Assets {
    fn load(&self, path: &str) -> Result<Option<Cow<'static, [u8]>>> {
        if let Some(bytes) = NucleusIcons.load(path)? {
            return Ok(Some(bytes));
        }
        KitAssets.load(path)
    }

    fn list(&self, path: &str) -> Result<Vec<SharedString>> {
        let mut all = KitAssets.list(path)?;
        all.extend(NucleusIcons.list(path)?);
        all.sort();
        all.dedup();
        Ok(all)
    }
}

/// 同梱フォント（デザインと同じ見た目にするため OS のフォントに頼らない）。
pub fn fonts() -> Vec<Cow<'static, [u8]>> {
    vec![
        Cow::Borrowed(include_bytes!("../../../assets/fonts/Inter-Regular.otf").as_slice()),
        Cow::Borrowed(include_bytes!("../../../assets/fonts/Inter-Medium.otf").as_slice()),
        Cow::Borrowed(include_bytes!("../../../assets/fonts/Inter-SemiBold.otf").as_slice()),
        Cow::Borrowed(include_bytes!("../../../assets/fonts/JetBrainsMono-Regular.ttf").as_slice()),
        Cow::Borrowed(include_bytes!("../../../assets/fonts/JetBrainsMono-Italic.ttf").as_slice()),
        Cow::Borrowed(include_bytes!("../../../assets/fonts/JetBrainsMono-Bold.ttf").as_slice()),
    ]
}
