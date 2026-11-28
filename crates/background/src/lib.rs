//! 外のプログラム（git、言語サーバー、拡張機能の `run-process` など）を起動する、ただ 1 つの入口。
//!
//! Nucleus は Windows ではコンソールを持たない GUI アプリなので、ふつうに起動すると
//! プログラムごとに黒いコンソール画面が開いては閉じる。ここで作る [`Command`] は画面を出さない。
//! `std::process::Command::new` を直接使うと clippy が止める（ルートの `clippy.toml`）。

use std::{ffi::OsStr, process::Command};

/// 画面を出さずに動かすプログラムのコマンド。
#[allow(clippy::disallowed_methods)]
pub fn command(program: impl AsRef<OsStr>) -> Command {
    let mut cmd = Command::new(program);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt as _;
        /// コンソール画面を作らない。
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }
    #[cfg(not(windows))]
    let _ = &mut cmd;
    cmd
}
