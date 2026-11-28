//! Nucleus の Extension を Rust で書くための SDK（core@1）。
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
pub mod bindings {
    wit_bindgen::generate!({
        path: "wit",
        world: "extension",
        pub_export_macro: true,
        default_bindings_module: "nucleus_extension_api::bindings",
    });
}

pub use bindings::nucleus::extension::host;
use bindings::Guest;


/// Extension が実装するもの。`activate` は省略できる。
pub trait Extension {
    fn activate() -> Result<(), String> {
        Ok(())
    }
    fn run_command(id: String) -> Result<(), String>;
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
}

/// Extension の型を Nucleus に公開する。
#[macro_export]
macro_rules! export_extension {
    ($t:ty) => {
        type __NucleusExtension = $crate::__Adapter<$t>;
        $crate::bindings::export!(__NucleusExtension with_types_in $crate::bindings);
    };
}
