use std::{
    env, fs,
    path::{Path, PathBuf},
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
    #[allow(dead_code)]
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
    let _app_manifest = cfg
        .profiles
        .get(&profile)
        .unwrap_or_else(|| panic!("profile '{profile}' not found in mquickjs.build.toml"))
        .app_manifest
        .clone();

    let _app_manifest = workspace_root.join(_app_manifest);

    let out_dir = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR not set"));
    let ridl_out = out_dir.join("ridl");
    fs::create_dir_all(&ridl_out).expect("create ridl out dir");

    // NOTE: mquickjs-sys build.rs does NOT run tool binaries.
    // Tools are executed via ridl-builder to keep builds fast and avoid ETXTBSY.

    let _ridl_extensions_enabled = env::var_os("CARGO_FEATURE_RIDL_EXTENSIONS").is_some();

    // Resolve build output directory (must be produced by ridl-builder).
    let target_dir = workspace_root.join("target");
    let target_triple = env::var("TARGET").expect("TARGET not set");
    let is_release = env::var("PROFILE").map(|p| p == "release").unwrap_or(false);
    let mode = if is_release { "release" } else { "debug" };

    let build_out_dir = target_dir
        .join("mquickjs-build")
        .join(&profile)
        .join(&target_triple)
        .join(mode)
        .join(if _ridl_extensions_enabled {
            "ridl"
        } else {
            "base"
        });

    let build_output_path = build_out_dir.join("mquickjs_build_output.json");
    if !build_output_path.exists() {
        panic!(
            "Missing mquickjs build outputs. Run: cargo run -p ridl-builder -- build-tools && cargo run -p ridl-builder -- build-mquickjs\nExpected: {}",
            build_output_path.display()
        );
    }

    let build_output = read_build_output(&build_output_path);
    if build_output.schema_version != 1 {
        panic!(
            "Unsupported mquickjs_build_output schema_version {}",
            build_output.schema_version
        );
    }

    // The build output json may carry workspace-root-relative paths (e.g.
    // when the framework artifacts are checked into an integrating tree for
    // portability). Resolve them against the mquickjs.build.toml directory
    // and canonicalize before emitting: downstream build scripts (e.g.
    // mquickjs-rs bindgen) run with their OWN package root as cwd, so the
    // emitted paths must be absolute.
    fn resolve(p: &Path, root: &Path) -> PathBuf {
        let joined = if p.is_absolute() {
            p.to_path_buf()
        } else {
            root.join(p)
        };
        joined.canonicalize().unwrap_or_else(|e| {
            // A relative escape here would be silently mis-consumed by any
            // downstream reader that runs from a different cwd — surface it
            // (e.g. MQUICKJS_LIB_DIR when lib/ is not checked in).
            println!(
                "cargo:warning=mquickjs-sys build: cannot canonicalize {} ({}); emitting as-is",
                joined.display(),
                e
            );
            joined
        })
    }

    println!("cargo:rerun-if-changed={}", cfg_path.display());

    for inp in &build_output.inputs {
        println!("cargo:rerun-if-changed={}", resolve(inp, &workspace_root).display());
    }

    // Expose include dir for downstream bindgen consumers.
    // Bindgen is performed in higher-level crates (e.g. mquickjs-rs), to keep this sys crate
    // focused on native build orchestration and linking.
    println!(
        "cargo:rustc-env=MQUICKJS_INCLUDE_DIR={}",
        resolve(&build_output.include_dir, &workspace_root).display()
    );

    // Expose native artifact locations for downstream crates to decide how to link.
    println!(
        "cargo:rustc-env=MQUICKJS_LIB_DIR={}",
        resolve(&build_output.lib_dir, &workspace_root).display()
    );

    // Do not emit any link directives from this sys crate.
    // mquickjs-rs is the canonical crate that owns native linking.
}

fn read_workspace_cfg(path: &Path) -> WorkspaceBuildConfig {
    let text = fs::read_to_string(path)
        .unwrap_or_else(|e| panic!("Failed to read {}: {e}", path.display()));
    toml::from_str(&text).unwrap_or_else(|e| panic!("Failed to parse {}: {e}", path.display()))
}

fn read_build_output(path: &Path) -> MquickjsBuildOutput {
    let text = fs::read_to_string(path)
        .unwrap_or_else(|e| panic!("Failed to read {}: {e}", path.display()));
    serde_json::from_str(&text)
        .unwrap_or_else(|e| panic!("Failed to parse {}: {e}", path.display()))
}

fn select_profile(cfg: &WorkspaceBuildConfig) -> String {
    cfg.default
        .clone()
        .unwrap_or_else(|| "framework".to_string())
}

fn find_mquickjs_build_toml() -> PathBuf {
    // External override. Note: this must be provided by the outer environment (shell, CI, or
    // workspace .cargo/config.toml). Build script emitted env vars do not propagate to other crates.
    println!("cargo:rerun-if-env-changed=MQUICKJS_BUILD_TOML");

    if let Ok(p) = env::var("MQUICKJS_BUILD_TOML") {
        let path = PathBuf::from(p);
        if !path.exists() {
            panic!(
                "MQUICKJS_BUILD_TOML points to a non-existent path: {}",
                path.display()
            );
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
