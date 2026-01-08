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

include!(concat!(env!("OUT_DIR"), "/bindings.rs"));
