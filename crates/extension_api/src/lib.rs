//! Nucleus の Extension を Rust で書くための SDK（core@1 と ui@1）。
//!
//! ```ignore
//! use nucleus_extension_api::{Extension, export_extension, host};
//!
//! struct Hello;
//! impl Extension for Hello {
//!     fn run_command(id: String) -> Result<(), String> {
//!         host::notify(host::NotificationLevel::Info, &format!("hello from {id}"));
//!         Ok(())
//!     }
//! }
//! export_extension!(Hello);
//! ```
//!
//! `cargo build --target wasm32-wasip2 --release` でビルドし、できた `.wasm` を
//! `extension.yaml` の `main` に書く。

#[doc(hidden)]
#[allow(clippy::too_many_arguments)]
pub mod bindings {
    wit_bindgen::generate!({
        path: "wit",
        world: "extension",
        pub_export_macro: true,
        default_bindings_module: "nucleus_extension_api::bindings",
    });
}

pub mod ui;

use bindings::Guest;
pub use bindings::nucleus::extension::events::WorkspaceEvent;
pub use bindings::nucleus::extension::host;
use bindings::nucleus::extension::ui as wit_ui;

/// Extension が実装するもの。使わないものは省略できる。
pub trait Extension {
    fn activate() -> Result<(), String> {
        Ok(())
    }
    fn run_command(id: String) -> Result<(), String> {
        Err(format!("unknown command `{id}`"))
    }
    fn on_event(_event: WorkspaceEvent) {}
    /// ビューの中身（ui@1）。[`ui::El`] で組み立てる。
    fn render(_view: String) -> ui::El {
        ui::v_stack()
    }
    fn handle_ui_event(_view: String, _key: String, _event: ui::UiEvent) -> Result<(), String> {
        Ok(())
    }
}

#[doc(hidden)]
pub struct __Adapter<T>(core::marker::PhantomData<T>);

impl<T: Extension> Guest for __Adapter<T> {
    fn activate() -> Result<(), String> {
        T::activate()
    }
    fn run_command(id: String) -> Result<(), String> {
        T::run_command(id)
    }
    fn on_event(event: WorkspaceEvent) {
        T::on_event(event)
    }
    fn render(view: String) -> Vec<wit_ui::Node> {
        T::render(view).flatten()
    }
    fn handle_ui_event(view: String, key: String, event: wit_ui::UiEvent) -> Result<(), String> {
        T::handle_ui_event(view, key, event)
    }
}

/// Extension の型を Nucleus に公開する。
#[macro_export]
macro_rules! export_extension {
    ($t:ty) => {
        type __NucleusExtension = $crate::__Adapter<$t>;
        $crate::bindings::export!(__NucleusExtension with_types_in $crate::bindings);
    };
}
