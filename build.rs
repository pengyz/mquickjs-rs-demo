use std::{env, path::{Path, PathBuf}};

fn main() {
    // Thin RIDL invoker: copy app-level aggregate outputs prepared by ridl-builder
    // into this crate's OUT_DIR so `include!(concat!(env!("OUT_DIR"), ...))` works.
    let out_dir = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR"));

    let workspace_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let aggregate_dir = workspace_root.join("target").join("ridl").join("aggregate");

    if !aggregate_dir.exists() {
        panic!(
            "Missing RIDL aggregate outputs. Run: cargo run -p ridl-builder -- prepare --profile framework\nExpected directory: {}",
            aggregate_dir.display()
        );
    }

    copy_required(&aggregate_dir, &out_dir, "ridl_symbols.rs");
    copy_required(&aggregate_dir, &out_dir, "ridl_slot_indices.rs");
    copy_required(&aggregate_dir, &out_dir, "ridl_ctx_ext.rs");
    copy_required(&aggregate_dir, &out_dir, "ridl_context_init.rs");
    copy_required(&aggregate_dir, &out_dir, "ridl_modules_initialize.rs");
    copy_required(&aggregate_dir, &out_dir, "ridl_initialize.rs");

    // Work around Cargo build-script fingerprinting: make sure app rebuilds when inputs change.
    println!(
        "cargo:rerun-if-changed={}",
        workspace_root.join("deps/ridl-tool").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        workspace_root.join("ridl-builder").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        workspace_root.join("Cargo.toml").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        workspace_root.join("ridl-modules").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        aggregate_dir.display()
    );
}

fn copy_required(from_dir: &Path, to_dir: &Path, file_name: &str) {
    let from = from_dir.join(file_name);
    if !from.exists() {
        panic!("Missing required RIDL file: {}", from.display());
    }

    let to = to_dir.join(file_name);
    std::fs::copy(&from, &to)
        .unwrap_or_else(|e| panic!("failed to copy {} -> {}: {e}", from.display(), to.display()));
}

