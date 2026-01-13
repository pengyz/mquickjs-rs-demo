mod aggregate;
mod config;
mod module_discovery;
mod probe_bindgen;

use std::{
    env,
    path::{Path, PathBuf},
    process::{Command, Stdio},
};

fn main() {
    let mut args = std::env::args().skip(1);
    let Some(cmd) = args.next() else {
        usage();
        std::process::exit(2);
    };

    match cmd.as_str() {
        "build-tools" => build_tools(),
        "build-mquickjs" => build_mquickjs(args.collect()),
        "aggregate" => aggregate_cmd(args.collect()),
        "prepare" => prepare_cmd(args.collect()),
        "probe-bindgen" => probe_bindgen::run(),
        _ => {
            eprintln!("Unknown command: {cmd}");
            usage();
            std::process::exit(2);
        }
    }
}

fn usage() {
    eprintln!("Usage: cargo run -p ridl-builder -- <command>");
    eprintln!("Commands:");
    eprintln!("  build-tools      Build internal tool binaries used by build.rs");
    eprintln!("  build-mquickjs   Build quickjs + generated headers (requires build-tools)");
    eprintln!("  aggregate        Generate ridl-manifest.json and mquickjs_ridl_register.h");
    eprintln!("  prepare          Build tools, aggregate RIDL, then build mquickjs with the aggregated header");
    eprintln!("  probe-bindgen    Compile a tiny crate to probe bindgen API");
}

fn aggregate_cmd(args: Vec<String>) {
    let (workspace_root, profile) = resolve_workspace_and_profile(&args);
    let modules = discover_modules(&workspace_root, &profile);

    let out = aggregate::aggregate(&workspace_root, &modules)
        .unwrap_or_else(|e| panic!("aggregate failed: {e}"));

    eprintln!("wrote {}", out.manifest_path.display());
    eprintln!("wrote {}", out.ridl_register_h.display());
}

fn find_workspace_root() -> PathBuf {
    // Walk up from crate dir until we find a Cargo.toml containing [workspace]
    let mut dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    loop {
        let cargo_toml = dir.join("Cargo.toml");
        if cargo_toml.exists() {
            if let Ok(content) = std::fs::read_to_string(&cargo_toml) {
                if content.contains("[workspace]") {
                    return dir;
                }
            }
        }
        if !dir.pop() {
            break;
        }
    }
    panic!("Unable to locate workspace root");
}

fn parse_opt(args: &[String], key: &str) -> Option<String> {
    let mut it = args.iter();
    while let Some(a) = it.next() {
        if a == key {
            return it.next().cloned();
        }
    }
    None
}

fn resolve_workspace_and_profile(args: &[String]) -> (PathBuf, String) {
    let profile = parse_opt(args, "--profile")
        .or_else(|| env::var("MQUICKJS_BUILD_PROFILE").ok())
        .unwrap_or_else(|| "framework".to_string());

    (find_workspace_root(), profile)
}

fn resolve_target_triple() -> String {
    // `cargo run` does not set TARGET by default (it builds for host), but our build layout
    // uses an explicit target triple directory. Prefer explicit overrides, then fall back to
    // the host triple.
    if let Ok(t) = env::var("TARGET") {
        return t;
    }
    if let Ok(t) = env::var("HOST") {
        return t;
    }
    // `rustc -vV` prints `host: <triple>`.
    let out = Command::new("rustc")
        .arg("-vV")
        .output()
        .expect("failed to run rustc -vV");
    let s = String::from_utf8_lossy(&out.stdout);
    for line in s.lines() {
        if let Some(rest) = line.strip_prefix("host: ") {
            return rest.trim().to_string();
        }
    }
    panic!("Unable to resolve target triple (no TARGET/HOST env, and rustc -vV has no host)");
}

fn discover_modules(workspace_root: &Path, profile: &str) -> Vec<aggregate::Module> {
    let build_toml = workspace_root.join("mquickjs.build.toml");
    let cfg = config::parse_mquickjs_build_toml(&build_toml);
    let prof = cfg
        .profiles
        .get(profile)
        .unwrap_or_else(|| panic!("profile '{profile}' not found in {}", build_toml.display()));

    let app_manifest = workspace_root.join(&prof.app_manifest);
    module_discovery::discover_ridl_modules(&app_manifest)
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
    let bin_dir = if profile == "release" {
        "target/release"
    } else {
        "target/debug"
    };
    eprintln!("Built tools under {bin_dir}/ (ridl-tool, mquickjs-build)");
}

fn build_mquickjs(args: Vec<String>) {
    // This command is intended to be run manually before `cargo build/test`.
    // It avoids executing tool binaries from within build.rs (ETXTBSY issues in some envs).

    let mut cmd = Command::new("cargo");
    cmd.arg("run").arg("-p").arg("mquickjs-build").arg("--");

    // Default args: build base QuickJS + stdlib (no RIDL extensions).
    // Allow passing through extra args (e.g. --ridl-register-h ...).
    if args.is_empty() {
        let target_triple = resolve_target_triple();
        let profile = env::var("PROFILE").unwrap_or_else(|_| "debug".to_string());
        let mode = if profile == "release" { "release" } else { "debug" };

        let out_dir = format!("target/mquickjs-build/framework/{target_triple}/{mode}");

        cmd.arg("build")
            .arg("--mquickjs-dir")
            .arg("deps/mquickjs")
            .arg("--out")
            .arg(out_dir);
    } else {
        cmd.args(args);
    }

    run(cmd);
}

fn prepare_cmd(args: Vec<String>) {
    let (workspace_root, profile) = resolve_workspace_and_profile(&args);

    // 1) build tool binaries
    build_tools();

    // 2) aggregate ridl modules into stable outputs
    let modules = discover_modules(&workspace_root, &profile);
    let out = aggregate::aggregate(&workspace_root, &modules)
        .unwrap_or_else(|e| panic!("aggregate failed: {e}"));
    eprintln!("wrote {}", out.manifest_path.display());
    eprintln!("wrote {}", out.ridl_register_h.display());

    let target_triple = resolve_target_triple();
    let cargo_profile = env::var("PROFILE").unwrap_or_else(|_| "debug".to_string());
    let mode = if cargo_profile == "release" { "release" } else { "debug" };
    let out_dir = format!("target/mquickjs-build/{profile}/{target_triple}/{mode}");

    // 3) build mquickjs using the aggregated register header
    let mut cmd = Command::new("cargo");
    cmd.arg("run")
        .arg("-p")
        .arg("mquickjs-build")
        .arg("--")
        .arg("build")
        .arg("--mquickjs-dir")
        .arg("deps/mquickjs")
        .arg("--ridl-register-h")
        .arg(out.ridl_register_h)
        .arg("--out")
        .arg(out_dir);
    run(cmd);

    // TODO: class-id rs generation will be added once the header is stabilized.
}

fn run(mut cmd: Command) {
    cmd.stdout(Stdio::inherit());
    cmd.stderr(Stdio::inherit());
    let status = cmd.status().expect("failed to run command");
    if !status.success() {
        std::process::exit(status.code().unwrap_or(1));
    }
}
