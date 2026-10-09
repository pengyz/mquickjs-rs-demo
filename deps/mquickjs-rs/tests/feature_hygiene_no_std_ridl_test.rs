//! Feature hygiene: `ridl-extensions` must build WITHOUT `std`.
//!
//! 背景（openvela QEMU aarch64 集成，计划 2026-10-09-openvela-qemu-arm64
//! §4 阶段 3.4）：裸机目标（`aarch64-unknown-none`）没有 std，而 RIDL
//! 扩展能力（console singleton 胶水、ridl runtime）必须在裸机上可用。
//!
//! 历史：`ridl-extensions` feature 隐含 `std`，因此
//! `no-std,ridl-extensions` 组合直接触发两 feature 的互斥
//! `compile_error!`。3.4 把 `ridl-extensions` 与模式 feature 解耦
//! （mode 由应用按目标选择：host/`std`，裸机/`no-std`）。
//!
//! 本文件锁定这一组合的可编译性：`cargo build` 以
//! `--no-default-features --features no-std,ridl-extensions` 显式发起
//! 一次嵌套构建（feature 组合无法在单 crate 编译内表达）。std 模式
//! （默认）的组合由 workspace 全量测试覆盖，此处不重复。

use std::path::Path;
use std::process::Command;

/// 剥离外层 cargo 注入的特征/RUSTFLAGS/集成 env —— 嵌套构建必须以自己的
/// feature 集合与 workspace 默认配置解析，否则外层 `cargo test` 的
/// `CARGO_FEATURE_*` 会污染内层 feature resolution，`MQJS_ENGINE_LINK` /
/// `MQUICKJS_*` 会把树内/端口集成的链接模式带进来。
fn sanitize(cmd: &mut Command) {
    for (k, _) in std::env::vars() {
        let strip = k.starts_with("CARGO_FEATURE")
            || matches!(
                k.as_str(),
                "RUSTFLAGS"
                    | "RUSTDOCFLAGS"
                    | "CARGO_ENCODED_RUSTFLAGS"
                    | "CARGO_BUILD_TARGET"
                    | "MQJS_ENGINE_LINK"
                    | "MQUICKJS_BUILD_TOML"
                    | "MQUICKJS_RIDL_TARGET_DIR"
                    | "MQUICKJS_RIDL_CARGO_TOML"
                    | "MQUICKJS_RIDL_TOOL"
            );
        if strip {
            cmd.env_remove(&k);
        }
    }
}

fn cargo_build(args: &[&str], pkg_crate: &str) {
    // Repo root workspace manifest（本 crate 位于 deps/mquickjs-rs/）。
    let manifest = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../Cargo.toml");
    assert!(manifest.exists(), "workspace manifest not found: {manifest:?}");

    let mut cmd = Command::new(env!("CARGO"));
    sanitize(&mut cmd);
    cmd.arg("build")
        .arg("--manifest-path")
        .arg(&manifest)
        .args(args)
        .env_remove("CARGO_MANIFEST_DIR"); // 防外层值泄漏到内层 build script

    let ok = cmd
        .status()
        .unwrap_or_else(|e| panic!("failed to spawn cargo for {pkg_crate}: {e}"))
        .success();
    assert!(
        ok,
        "{pkg_crate} must build with `{}` (feature hygiene, see module docs)",
        args.join(" ")
    );
}

#[test]
fn mquickjs_rs_builds_no_std_with_ridl_extensions() {
    cargo_build(
        &[
            "-p",
            "mquickjs-rs",
            "--no-default-features",
            "--features",
            "no-std,ridl-extensions",
        ],
        "mquickjs-rs",
    );
}

#[test]
fn stdlib_module_builds_no_std() {
    // RIDL stdlib 模块（console singleton 的实现 crate）同锁：裸机模式可编译。
    cargo_build(
        &[
            "-p",
            "stdlib",
            "--no-default-features",
            "--features",
            "no-std",
        ],
        "stdlib",
    );
}
