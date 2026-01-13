//! mquickjs-sys: core FFI bindings to mquickjs

// bindgen output is noisy and not actionable for this project.
#![allow(
    non_camel_case_types,
    non_snake_case,
    non_upper_case_globals,
    dead_code,
    improper_ctypes,
    improper_ctypes_definitions,
    clippy::all
)]

pub fn include_dir() -> std::path::PathBuf {
    // Exported by deps/mquickjs-sys/build.rs as an absolute path.
    // Use an owned PathBuf so downstream crates can rely on it regardless of their cwd.
    std::path::PathBuf::from(env!("MQUICKJS_INCLUDE_DIR"))
}

pub fn header_path() -> std::path::PathBuf {
    include_dir().join("mquickjs.h")
}
