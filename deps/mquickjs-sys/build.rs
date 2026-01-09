use std::{
    env,
    fs,
    path::{Path, PathBuf},
    process::{Command, Stdio},
};

use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct WorkspaceBuildConfig {
    default: Option<String>,
    profiles: std::collections::BTreeMap<String, ProfileConfig>,
}

#[derive(Debug, Deserialize)]
struct ProfileConfig {
    app_manifest: String,
}

#[derive(Debug, Deserialize)]
struct MquickjsBuildOutput {
    schema_version: u32,
    lib_dir: PathBuf,
    include_dir: PathBuf,
    libs: Vec<String>,
    inputs: Vec<PathBuf>,
}

fn main() {
    let cfg_path = find_mquickjs_build_toml();
    let workspace_root = cfg_path
        .parent()
        .expect("mquickjs.build.toml must have a parent directory")
        .to_path_buf();

    let cfg = read_workspace_cfg(&cfg_path);

    let profile = select_profile(&cfg);
    let app_manifest = cfg
        .profiles
        .get(&profile)
        .unwrap_or_else(|| panic!("profile '{profile}' not found in mquickjs.build.toml"))
        .app_manifest
        .clone();

    let app_manifest = workspace_root.join(app_manifest);

    let out_dir = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR not set"));
    let ridl_out = out_dir.join("ridl");
    fs::create_dir_all(&ridl_out).expect("create ridl out dir");

    // Resolve tool binaries (must be built via `cargo run -p xtask -- build-tools`).
    let tools_dir = workspace_root.join("target").join(env::var("PROFILE").unwrap_or_else(|_| "debug".to_string()));
    let jidl_tool_bin = tools_dir.join(tool_exe_name("jidl-tool"));
    let mquickjs_build_bin = tools_dir.join(tool_exe_name("mquickjs-build"));

    if !jidl_tool_bin.exists() || !mquickjs_build_bin.exists() {
        panic!(
            "Missing tool binaries. Run: cargo run -p xtask -- build-tools\nExpected: {} and {}",
            jidl_tool_bin.display(),
            mquickjs_build_bin.display()
        );
    }

    // 1) ridl-tool resolve
    let plan_path = ridl_out.join("ridl_plan.json");
    let mut cmd = Command::new(&jidl_tool_bin);
    cmd.arg("resolve")
        .arg("--cargo-toml")
        .arg(&app_manifest)
        .arg("--out")
        .arg(&plan_path);
    run(cmd);

    // 2) ridl-tool generate
    let mut cmd = Command::new(&jidl_tool_bin);
    cmd.arg("generate")
        .arg("--plan")
        .arg(&plan_path)
        .arg("--out")
        .arg(&ridl_out);
    run(cmd);

    // 3) mquickjs-build build
    let target_dir = workspace_root.join("target");
    let target_triple = env::var("TARGET").expect("TARGET not set");
    let is_release = env::var("PROFILE").map(|p| p == "release").unwrap_or(false);
    let mode = if is_release { "release" } else { "debug" };

    let build_out_dir = target_dir
        .join("mquickjs-build")
        .join(&profile)
        .join(&target_triple)
        .join(mode);

    fs::create_dir_all(&build_out_dir).expect("create build out dir");

    let mquickjs_dir = workspace_root.join("deps/mquickjs");

    let mut cmd = Command::new(&mquickjs_build_bin);
    cmd.arg("build")
        .arg("--mquickjs-dir")
        .arg(&mquickjs_dir)
        .arg("--plan")
        .arg(&plan_path)
        .arg("--out")
        .arg(&build_out_dir);
    run(cmd);

    let build_output_path = build_out_dir.join("mquickjs_build_output.json");
    let build_output = read_build_output(&build_output_path);
    if build_output.schema_version != 1 {
        panic!("Unsupported mquickjs_build_output schema_version {}", build_output.schema_version);
    }

    println!("cargo:rerun-if-changed={}", cfg_path.display());
    // We can't reliably enumerate profile features dynamically; always rerun when mquickjs-profile changes.
    println!("cargo:rerun-if-changed={}", workspace_root.join("mquickjs-profile/Cargo.toml").display());

    for inp in &build_output.inputs {
        println!("cargo:rerun-if-changed={}", inp.display());
    }

    // bindgen against the build output include dir
    let bindings = bindgen::Builder::default()
        .header(build_output.include_dir.join("mquickjs.h").to_string_lossy())
        .clang_arg("-I")
        .clang_arg(build_output.include_dir.to_string_lossy())
        .clang_arg("-include")
        .clang_arg("stddef.h")
        .allowlist_recursively(true)
        .rust_edition(bindgen::RustEdition::Edition2024)
        .generate()
        .expect("Unable to generate bindings");

    bindings
        .write_to_file(out_dir.join("bindings.rs"))
        .expect("Couldn't write bindings!");

    println!(
        "cargo:rustc-link-search=native={}",
        build_output.lib_dir.display()
    );
    for lib in build_output.libs {
        println!("cargo:rustc-link-lib=static={lib}");
    }
}

fn read_workspace_cfg(path: &Path) -> WorkspaceBuildConfig {
    let text = fs::read_to_string(path)
        .unwrap_or_else(|e| panic!("Failed to read {}: {e}", path.display()));
    toml::from_str(&text)
        .unwrap_or_else(|e| panic!("Failed to parse {}: {e}", path.display()))
}

fn read_build_output(path: &Path) -> MquickjsBuildOutput {
    let text = fs::read_to_string(path)
        .unwrap_or_else(|e| panic!("Failed to read {}: {e}", path.display()));
    serde_json::from_str(&text)
        .unwrap_or_else(|e| panic!("Failed to parse {}: {e}", path.display()))
}

fn select_profile(cfg: &WorkspaceBuildConfig) -> String {
    // Prefer an explicitly enabled profile feature (from mquickjs-profile crate).
    // Cargo exposes enabled features to build scripts as env vars: CARGO_FEATURE_<FEATURE_NAME>.
    let candidates = ["framework", "tooling", "tests"];
    for c in candidates {
        let key = format!("CARGO_FEATURE_{}", c.to_uppercase().replace('-', "_"));
        if env::var_os(key).is_some() {
            return c.to_string();
        }
    }

    cfg.default.clone().unwrap_or_else(|| "framework".to_string())
}

fn tool_exe_name(base: &str) -> String {
    if cfg!(windows) {
        format!("{base}.exe")
    } else {
        base.to_string()
    }
}

fn find_mquickjs_build_toml() -> PathBuf {
    // External override. Note: this must be provided by the outer environment (shell, CI, or
    // workspace .cargo/config.toml). Build script emitted env vars do not propagate to other crates.
    println!("cargo:rerun-if-env-changed=MQUICKJS_BUILD_TOML");

    if let Ok(p) = env::var("MQUICKJS_BUILD_TOML") {
        let path = PathBuf::from(p);
        if !path.exists() {
            panic!("MQUICKJS_BUILD_TOML points to a non-existent path: {}", path.display());
        }
        return path;
    }

    // Default: discover workspace root by walking up from this crate's manifest dir and
    // finding a Cargo.toml that declares a [workspace].
    let mut dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    loop {
        let cargo_toml = dir.join("Cargo.toml");
        if cargo_toml.exists() {
            if let Ok(content) = fs::read_to_string(&cargo_toml) {
                if content.contains("[workspace]") {
                    let cfg = dir.join("mquickjs.build.toml");
                    if cfg.exists() {
                        return cfg;
                    }
                }
            }
        }

        if !dir.pop() {
            break;
        }
    }

    panic!(
        "Unable to locate mquickjs.build.toml. Set MQUICKJS_BUILD_TOML to an absolute path, \
or ensure this crate is built within a workspace root that contains mquickjs.build.toml."
    );
}

fn run(mut cmd: Command) {
    cmd.stdout(Stdio::inherit());
    cmd.stderr(Stdio::inherit());

    let status = cmd.status().expect("failed to run command");
    if !status.success() {
        panic!("command failed: {status}");
    }
}
