use std::{env, path::PathBuf};

extern crate mquickjs_sys;

fn main() {
    let out_path = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR"));

    // Own native linking here (single source of truth for consumers).
    // The sys crate exposes the actual build output location.
    let lib_dir = mquickjs_sys::include_dir()
        .parent()
        .expect("include_dir has parent")
        .join("lib");
    println!("cargo:rustc-link-search=native={}", lib_dir.display());
    println!("cargo:rustc-link-lib=static=mquickjs");

    // Generate FFI bindings for the headers produced by mquickjs-build.
    let bindings = bindgen::Builder::default()
        .header(mquickjs_sys::header_path().to_string_lossy())
        .clang_arg("-I")
        .clang_arg(mquickjs_sys::include_dir().to_string_lossy())
        .clang_arg("-include")
        .clang_arg("stddef.h")
        .parse_callbacks(Box::new(bindgen::CargoCallbacks::new()))
        .allowlist_recursively(true)
        .rust_edition(bindgen::RustEdition::Edition2024)
        .generate()
        .expect("Unable to generate bindings");

    bindings
        .write_to_file(out_path.join("bindings.rs"))
        .expect("Failed to write bindings");
}
