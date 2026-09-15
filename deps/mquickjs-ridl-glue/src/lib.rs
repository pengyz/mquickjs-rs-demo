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

/// 应用侧（拥有完整 RIDL 模块集合的**叶子**二进制）链接 ridl 变体的 C stdlib。
///
/// # 为什么需要它
///
/// 应用的 C stdlib（`mqjs_stdlib_impl.o`）里的 `js_c_function_table` 会**强引用**
/// 该应用全部 RIDL 模块的 C 入口符号。哪个二进制需要 RIDL 是**叶子**的属性，
/// 不是全局 feature 的属性 —— 因此由各叶子自行声明：
///
/// - **应用**（本函数）：链接 ridl stdlib → RIDL 完整可用
/// - **`mquickjs-rs` 自身的 test 目标**：由其 `build.rs` 的
///   `rustc-link-arg-tests` 固定链接 base stdlib → 不引用任何 RIDL 符号
///
/// 两者互不干扰，因为 `rustc-link-arg-*` 只作用于发出指令的包自己的目标，
/// 不会随 rlib 传播给依赖方。
///
/// # 何时调用
///
/// 在应用自己的 `build.rs` 中调用（通常紧跟 `ridl-tool module` 之后）。
pub fn emit_native_stdlib_link() {
    let target_dir = resolve_target_dir();

    let triple = env::var("TARGET").expect("TARGET is set by cargo for build scripts");
    let mode = match env::var("PROFILE").as_deref() {
        Ok("release") => "release",
        _ => "debug",
    };

    // 与 ridl-builder 写入产物的路径保持一致：
    //   target/mquickjs-build/framework/<triple>/<mode>/{base,ridl}
    let lib_dir = target_dir
        .join("mquickjs-build")
        .join("framework")
        .join(&triple)
        .join(mode)
        .join("ridl")
        .join("lib");

    let stdlib = lib_dir.join("libmquickjs_stdlib_ridl.a");
    if !stdlib.exists() {
        panic!(
            "Missing RIDL native stdlib. Run: cargo run -p ridl-builder -- prepare\nExpected: {}",
            stdlib.display()
        );
    }

    println!("cargo:rerun-if-changed={}", stdlib.display());

    // 用 `--whole-archive` 强制包含归档全部成员，而不是 `-L<dir> -lmquickjs_stdlib_ridl`。
    //
    // 原因：由 `rustc-link-lib` 产生的 `-l` 会被 cargo 放在 **rlib 之前**。
    // 静态归档只在"扫描到它时存在未定义符号"的情况下才会抽出成员，
    // 因此放在 rlib 之前会导致 `mqjs_stdlib_impl.o`（定义 `js_stdlib` 与
    // `js_c_function_table`）不被抽出 → 后续 `undefined symbol: js_stdlib`。
    //
    // `--whole-archive` 无条件包含全部成员，从而消除对库顺序的依赖；
    // 同时 `mquickjs_ridl_register.o` 也被强制包含，它对 RIDL 模块 C 入口的
    // 未定义引用由随后的模块 rlib 满足。
    //
    // 整个参数写成**单个** `-Wl,...`，以保证三个子参数在命令行中相邻且有序。
    println!(
        "cargo:rustc-link-arg=-Wl,--whole-archive,{},--no-whole-archive",
        stdlib.display()
    );
}

/// 解析本应用构建产物所在的 target 目录。
///
/// 优先级与 `emit()` 一致：env override > 配置文件 > cargo metadata。
fn resolve_target_dir() -> PathBuf {
    let cfg = find_and_parse_config();

    if let Ok(dir) = env::var("MQUICKJS_RIDL_TARGET_DIR") {
        return PathBuf::from(dir);
    }
    if let Some(dir) = cfg.as_ref().and_then(|c| c.target_dir.clone()) {
        return dir;
    }

    let cargo_toml = env::var("MQUICKJS_RIDL_CARGO_TOML")
        .map(PathBuf::from)
        .ok()
        .or_else(|| cfg.as_ref().map(|c| c.root_dir.join("Cargo.toml")))
        .unwrap_or_else(|| {
            panic!(
                "Unable to locate root Cargo.toml. Provide MQUICKJS_RIDL_CARGO_TOML or create mquickjs.ridl.toml in an ancestor directory."
            )
        });
    let cargo_toml = cargo_toml
        .canonicalize()
        .expect("canonicalize root Cargo.toml");

    cargo_metadata(&cargo_toml).target_directory.clone()
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
    copy_required(&aggregate_dir, &out_dir, "ridl_context_ext.rs");
    copy_required(&aggregate_dir, &out_dir, "ridl_bootstrap.rs");

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
        panic!(
            "unsupported mquickjs.ridl.toml version={}, path={}",
            raw.version,
            path.display()
        );
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
