use std::{
    env,
    path::{Path, PathBuf},
};

#[derive(Debug, Clone)]
pub struct Config {
    pub path: PathBuf,
    pub root_dir: PathBuf,
    pub target_dir: Option<PathBuf>,
    pub app_id: Option<String>,
}

pub fn emit() {
    let out_dir = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR"));

    let cfg = find_and_parse_config();

    // SoT: root Cargo.toml.
    // Priority: env override > config file.
    let cargo_toml = env::var("MQUICKJS_RIDL_CARGO_TOML")
        .map(PathBuf::from)
        .ok()
        .or_else(|| cfg.as_ref().map(|c| c.root_dir.join("Cargo.toml")))
        .unwrap_or_else(|| {
            panic!(
                "Unable to locate root Cargo.toml. Provide MQUICKJS_RIDL_CARGO_TOML or create mquickjs.ridl.toml in an ancestor directory."
            )
        })
        .canonicalize()
        .expect("canonicalize root Cargo.toml");

    let meta = cargo_metadata(&cargo_toml);

    let target_dir = env::var("MQUICKJS_RIDL_TARGET_DIR")
        .map(PathBuf::from)
        .ok()
        .or_else(|| cfg.as_ref().and_then(|c| c.target_dir.clone()))
        .unwrap_or_else(|| meta.target_directory.clone());

    let root_pkg = select_root_package(&meta, &cargo_toml);

    let app_id = env::var("MQUICKJS_RIDL_APP_ID")
        .ok()
        .or_else(|| cfg.as_ref().and_then(|c| c.app_id.clone()))
        .unwrap_or_else(|| normalize_app_id(&root_pkg.name));

    let aggregate_dir = target_dir
        .join("ridl")
        .join("apps")
        .join(app_id)
        .join("aggregate");

    if !aggregate_dir.exists() {
        panic!(
            "Missing RIDL aggregate outputs. Run: cargo run -p ridl-builder -- aggregate --cargo-toml {} --intent build\nExpected directory: {}",
            cargo_toml.display(),
            aggregate_dir.display()
        );
    }

    copy_required(&aggregate_dir, &out_dir, "ridl_symbols.rs");
    copy_required(&aggregate_dir, &out_dir, "ridl_slot_indices.rs");
    copy_required(&aggregate_dir, &out_dir, "ridl_ctx_ext.rs");
    copy_required(&aggregate_dir, &out_dir, "ridl_context_init.rs");
    copy_required(&aggregate_dir, &out_dir, "ridl_modules_initialize.rs");
    copy_required(&aggregate_dir, &out_dir, "ridl_initialize.rs");

    println!("cargo:rerun-if-changed={}", cargo_toml.display());
    if let Some(cfg) = cfg {
        println!("cargo:rerun-if-changed={}", cfg.path.display());
    }
    println!("cargo:rerun-if-changed={}", aggregate_dir.display());
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

fn normalize_app_id(s: &str) -> String {
    s.chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || ch == '_' {
                ch
            } else {
                '_'
            }
        })
        .collect()
}

#[derive(serde::Deserialize)]
struct CargoMetadata {
    packages: Vec<CargoPackage>,
    target_directory: PathBuf,
}

#[derive(serde::Deserialize, Clone)]
struct CargoPackage {
    name: String,
    manifest_path: PathBuf,
}

fn cargo_metadata(manifest_path: &Path) -> CargoMetadata {
    let mut cmd = std::process::Command::new("cargo");
    cmd.arg("metadata")
        .arg("--format-version=1")
        .arg("--manifest-path")
        .arg(manifest_path);

    let out = cmd.output().expect("failed to run cargo metadata");
    if !out.status.success() {
        panic!(
            "cargo metadata failed (exit={:?}): {}",
            out.status.code(),
            String::from_utf8_lossy(&out.stderr)
        );
    }

    serde_json::from_slice(&out.stdout).expect("parse cargo metadata json")
}

fn select_root_package(meta: &CargoMetadata, cargo_toml: &Path) -> CargoPackage {
    let want = cargo_toml
        .canonicalize()
        .unwrap_or_else(|e| panic!("failed to canonicalize {}: {e}", cargo_toml.display()));

    meta.packages
        .iter()
        .find(|p| p.manifest_path == want)
        .cloned()
        .unwrap_or_else(|| panic!("root package not found for {}", want.display()))
}

fn find_and_parse_config() -> Option<Config> {
    let mut dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR"));

    loop {
        let cfg_path = dir.join("mquickjs.ridl.toml");
        if cfg_path.exists() {
            return Some(parse_config(&cfg_path));
        }

        if !dir.pop() {
            return None;
        }
    }
}

fn parse_config(path: &Path) -> Config {
    #[derive(serde::Deserialize)]
    struct Raw {
        version: u32,
        target_dir: Option<PathBuf>,
        app_id: Option<String>,
        intent: Option<String>,
    }

    let content = std::fs::read_to_string(path)
        .unwrap_or_else(|e| panic!("failed to read {}: {e}", path.display()));

    let raw: Raw = toml::from_str(&content)
        .unwrap_or_else(|e| panic!("failed to parse {}: {e}", path.display()));

    if raw.version != 1 {
        panic!("unsupported mquickjs.ridl.toml version={}, path={}", raw.version, path.display());
    }

    let root_dir = path
        .parent()
        .unwrap_or_else(|| panic!("config has no parent dir: {}", path.display()))
        .to_path_buf();

    if !root_dir.join("Cargo.toml").exists() {
        panic!(
            "mquickjs.ridl.toml must be placed next to root Cargo.toml. missing: {}",
            root_dir.join("Cargo.toml").display()
        );
    }

    let _ = raw.intent;

    Config {
        path: path.to_path_buf(),
        root_dir,
        target_dir: raw.target_dir,
        app_id: raw.app_id,
    }
}
