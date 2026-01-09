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

pub fn include_dir() -> &'static std::path::Path {
    std::path::Path::new(env!("MQUICKJS_INCLUDE_DIR"))
}

pub fn header_path() -> std::path::PathBuf {
    include_dir().join("mquickjs.h")
}

