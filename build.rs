use std::{env, path::PathBuf, process::Command};

fn main() {
    // Thin RIDL invoker: delegate all RIDL aggregation/generation to ridl-tool.
    let out_dir = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR"));

    let profile = env::var("PROFILE").unwrap_or_else(|_| "debug".to_string());
    let workspace_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));

    let tool = workspace_root
        .join("target")
        .join(&profile)
        .join(tool_exe_name("ridl-tool"));

    if !tool.exists() {
        panic!(
            "Missing tool binary. Run: cargo run -p xtask -- build-tools\nExpected: {}",
            tool.display()
        );
    }

    let plan_path = out_dir.join("ridl_plan.json");

    run(Command::new(&tool)
        .arg("resolve")
        .arg("--cargo-toml")
        .arg(workspace_root.join("Cargo.toml"))
        .arg("--out")
        .arg(&plan_path));

    run(Command::new(&tool)
        .arg("generate")
        .arg("--plan")
        .arg(&plan_path)
        .arg("--out")
        .arg(&out_dir));

    // Work around Cargo build-script fingerprinting: make sure app rebuilds when ridl-tool changes.
    println!(
        "cargo:rerun-if-changed={}",
        workspace_root.join("deps/ridl-tool").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        workspace_root.join("Cargo.toml").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        workspace_root.join("ridl-modules").display()
    );
}

fn tool_exe_name(base: &str) -> String {
    if cfg!(windows) {
        format!("{base}.exe")
    } else {
        base.to_string()
    }
}

fn run(cmd: &mut Command) {
    let status = cmd
        .status()
        .unwrap_or_else(|e| panic!("failed to spawn: {e}"));
    if !status.success() {
        panic!("command failed: {cmd:?}");
    }
}
