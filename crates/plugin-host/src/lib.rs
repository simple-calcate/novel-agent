//! 插件宿主：清单解析、权限评估、桌面 WASM 运行时。

pub mod manifest;
pub mod permissions;
pub mod runtime;

pub use manifest::*;
pub use permissions::*;
pub use runtime::*;
