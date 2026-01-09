mod probe_bindgen;

use std::process::{Command, Stdio};

fn main() {
    let mut args = std::env::args().skip(1);
    let Some(cmd) = args.next() else {
        usage();
        std::process::exit(2);
    };

    match cmd.as_str() {
        "build-tools" => build_tools(),
        "probe-bindgen" => probe_bindgen::run(),
        _ => {
            eprintln!("Unknown command: {cmd}");
            usage();
            std::process::exit(2);
        }
    }
}

fn usage() {
    eprintln!("Usage: cargo run -p xtask -- <command>");
    eprintln!("Commands:");
    eprintln!("  build-tools      Build internal tool binaries used by build.rs");
    eprintln!("  probe-bindgen    Compile a tiny crate to probe bindgen API");
}

fn build_tools() {
    // Build tool binaries in one cargo invocation to avoid repeated locking.
    let mut cmd = Command::new("cargo");
    cmd.arg("build")
        .arg("-p")
        .arg("ridl-tool")
        .arg("-p")
        .arg("mquickjs-build");
    run(cmd);

    // Print their expected locations for convenience.
    let profile = std::env::var("PROFILE").unwrap_or_else(|_| "debug".to_string());
    let bin_dir = if profile == "release" { "target/release" } else { "target/debug" };
    eprintln!("Built tools under {bin_dir}/ (ridl-tool, mquickjs-build)");
}

fn run(mut cmd: Command) {
    cmd.stdout(Stdio::inherit());
    cmd.stderr(Stdio::inherit());
    let status = cmd.status().expect("failed to run command");
    if !status.success() {
        std::process::exit(status.code().unwrap_or(1));
    }
}
