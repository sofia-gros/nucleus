//! gpui のアプリの中で、いまのホストへの窓口を持っておく。

use gpui_kit::{App, Global};

use crate::Client;

struct GlobalClient(Client);

impl Global for GlobalClient {}

/// いまのホストへの窓口を登録する（起動時に 1 回）。
pub fn set_global(client: Client, cx: &mut App) {
    cx.set_global(GlobalClient(client));
}

pub trait RemoteExt {
    /// いまのホストへの窓口。
    fn remote(&self) -> Client;
}

impl RemoteExt for App {
    fn remote(&self) -> Client {
        self.global::<GlobalClient>().0.clone()
    }
}
