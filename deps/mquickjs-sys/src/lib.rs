//! mquickjs-sys: core FFI bindings to mquickjs
//!
//! 本 crate 的 FFI 声明本身与平台无关。只有两个**构建期辅助函数**
//! （`include_dir` / `header_path`）需要 `std` —— 它们仅供下游的
//! `build.rs` 使用，而构建脚本始终以宿主的 std 编译。
//!
//! 因此 `no-std` 目标下关闭 `std` feature 即可。

// bindgen output is noisy and not actionable for this project.
#![cfg_attr(not(feature = "std"), no_std)]
#![allow(
    non_camel_case_types,
    non_snake_case,
    non_upper_case_globals,
    dead_code,
    improper_ctypes,
    improper_ctypes_definitions,
    clippy::all
)]

#[cfg(feature = "std")]
pub fn include_dir() -> std::path::PathBuf {
    // Exported by deps/mquickjs-sys/build.rs as an absolute path.
    // Use an owned PathBuf so downstream crates can rely on it regardless of their cwd.
    std::path::PathBuf::from(env!("MQUICKJS_INCLUDE_DIR"))
}

#[cfg(feature = "std")]
pub fn header_path() -> std::path::PathBuf {
    include_dir().join("mquickjs.h")
}
