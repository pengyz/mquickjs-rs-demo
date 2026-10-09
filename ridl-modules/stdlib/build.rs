//! Build script for the stdlib RIDL module.
//!
//! Two modes for producing the module glue (api.rs + glue.rs) into OUT_DIR:
//!
//! 1. **Host tool (authoritative)** — when `MQUICKJS_RIDL_TOOL` is set, run
//!    `ridl-tool module src/stdlib.ridl <out_dir>` (the SDK development
//!    flow; the tool is produced by `cargo run -p ridl-builder -- prepare`).
//! 2. **Pregenerated** — when the env is NOT set and `generated/{api,glue}.rs`
//!    exist next to this file, copy them. The generation is a pure function
//!    of src/stdlib.ridl (verified byte-deterministic), so the checked-in
//!    glue stays valid while the .ridl source is unchanged. This mode lets
//!    integrating trees (e.g. the OpenVela in-tree app) build without any
//!    prebuilt host tool.
//!
//! If neither is available, fail with an actionable hint.

use std::{env, path::PathBuf, process::Command};

fn main() {
    println!("cargo:rerun-if-changed=src/stdlib.ridl");

    let out_dir = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR not set"));

    let ridl_tool = env::var("MQUICKJS_RIDL_TOOL")
        .ok()
        .filter(|v| !v.is_empty())
        .map(PathBuf::from);
    if let Some(ridl_tool) = ridl_tool {
        println!("cargo:rerun-if-changed={}", ridl_tool.display());

        let status = Command::new(&ridl_tool)
            .arg("module")
            .arg("src/stdlib.ridl")
            .arg(&out_dir)
            .status()
            .unwrap_or_else(|e| panic!("failed to run {}: {e}", ridl_tool.display()));

        if !status.success() {
            panic!("ridl-tool failed (exit={:?})", status.code());
        }
        return;
    }

    let manifest_dir = env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR not set");
    let generated_dir = PathBuf::from(&manifest_dir).join("generated");

    let has_pregen = generated_dir.join("api.rs").try_exists().unwrap_or(false)
        && generated_dir.join("glue.rs").try_exists().unwrap_or(false);

    if !has_pregen {
        panic!(
            "stdlib module glue unavailable: set MQUICKJS_RIDL_TOOL to a ridl-tool \
             binary, or check in generated/api.rs + generated/glue.rs \
             (generate via `ridl-tool module src/stdlib.ridl <dir>`)"
        );
    }

    let ridl_mtime = std::fs::metadata("src/stdlib.ridl")
        .and_then(|m| m.modified())
        .ok();
    for file in ["api.rs", "glue.rs"] {
        let rel = format!("generated/{file}");
        println!("cargo:rerun-if-changed={rel}");
        if let Some(ridl) = ridl_mtime {
            let stale = std::fs::metadata(&generated_dir.join(file))
                .and_then(|m| m.modified())
                .map(|gen_mtime| gen_mtime < ridl)
                .unwrap_or(true);
            if stale {
                println!(
                    "cargo:warning=stdlib: {rel} is older than src/stdlib.ridl -- \
                     regenerate via the MQUICKJS_RIDL_TOOL flow"
                );
            }
        }
        let src = generated_dir.join(file);
        let dst = out_dir.join(file);
        std::fs::copy(&src, &dst)
            .unwrap_or_else(|e| panic!("failed to copy {} -> {}: {e}", src.display(), dst.display()));
    }
}
