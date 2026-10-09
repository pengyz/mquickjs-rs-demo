//! RIDL stdlib 模块（console singleton）。
//!
//! 运行模式与 mquickjs-rs 同构（feature 卫生，见
//! docs/planning/2026/2026-10-09-openvela-qemu-arm64.md §4 3.4）：
//!
//! - `std`（默认）：宿主/POSIX 环境，console 输出走 `print!`/`eprint!`；
//! - `no-std`：裸机（`aarch64-unknown-none` 等），console 输出走 FFI
//!   `printf` 桥（镜像 libc 的 stdout，即 NuttX NSH 控制台）。
//!
//! 生成的 glue（`rust_glue.rs.j2`）通过 `mquickjs_rs::glue_prelude`
//! 引入 `CString`/`Vec`/`Box` 等，两种模式共用同一份生成物。
#![cfg_attr(feature = "no-std", no_std)]
// `std` 与 `no-std` 互斥：同开会把 std-only 输出路径编进 no_std crate。
#[cfg(all(feature = "std", feature = "no-std"))]
compile_error!("features `std` and `no-std` are mutually exclusive");

// alloc 在 std 与 no_std 两种模式下都显式声明（与 mquickjs-rs 同构）：
// stdlib_impl 的 Vec/Box 直接使用，生成的 glue 经 mquickjs_rs::glue_prelude
// 间接使用。
extern crate alloc;

mquickjs_rs::ridl_include_module!();

pub mod impls {
    pub use crate::api::ConsoleSingleton;

    pub use crate::stdlib_impl::DefaultConsoleSingleton;

    pub use crate::stdlib_impl::create_console_singleton;
}

mod stdlib_impl;
