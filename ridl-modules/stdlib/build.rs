use std::{
    env,
    path::PathBuf,
    process::Command,
};

fn main() {
    println!("cargo:rerun-if-changed=src/stdlib.ridl");
    println!("cargo:rerun-if-env-changed=MQUICKJS_RIDL_TOOL");

    let out_dir = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR not set"));

    let ridl_tool = resolve_ridl_tool();

    // Generate module-level glue: api.rs + glue.rs
    let status = Command::new(&ridl_tool)
        .arg("module")
        .arg("src/stdlib.ridl")
        .arg(&out_dir)
        .status()
        .unwrap_or_else(|e| panic!("failed to run {}: {e}", ridl_tool.display()));

    if !status.success() {
        panic!("ridl-tool failed (exit={:?})", status.code());
    }
}

fn resolve_ridl_tool() -> PathBuf {
    env::var("MQUICKJS_RIDL_TOOL")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            panic!(
                "MQUICKJS_RIDL_TOOL is not set. Hint: run `cargo run -p ridl-builder -- prepare` first."
            )
        })
}
