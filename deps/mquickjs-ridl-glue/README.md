# mquickjs-ridl-glue

该 crate 负责把 ridl-builder 生成的 **app/root 级 RIDL 聚合产物**接到 Rust 编译链里：

- `build.rs`：从 `<target_dir>/ridl/apps/<app-id>/aggregate/` 拷贝必须的 `*.rs` 到 `OUT_DIR`，供上层通过 `include!(concat!(env!("OUT_DIR"), ...))` 使用。
- `lib.rs`：仅作为薄封装，转调 `mquickjs-rs` 的初始化入口（process/context）。

## 约束

- RIDL 注册必须是 **编译期** 完成（不能运行期动态注册 QuickJS C API）。
- 不做“模块白名单/硬编码”；模块选择由 root crate 的 Cargo 依赖（direct deps）决定。

## 环境变量

- `MQUICKJS_RIDL_CARGO_TOML`：指定用于 `cargo metadata` 的根 `Cargo.toml`（建议由 root crate 的 build.rs 注入；否则可在 shell 中 export）。
- `MQUICKJS_RIDL_TARGET_DIR`：覆盖 `cargo metadata.target_directory`，用于多 repo/多使用方共享输出目录。
- `MQUICKJS_RIDL_APP_ID`：覆盖 app-id（默认：对 root package.name 做规范化：将所有非 `[A-Za-z0-9_]` 字符替换为 `_`）。

## 预期工作流

1. 先生成聚合产物：
   
   ```bash
   cargo run -p ridl-builder -- aggregate --cargo-toml /abs/path/to/Cargo.toml --intent build
   ```

2. 再编译 root crate（本 crate 的 build.rs 会去拷贝聚合产物）。
