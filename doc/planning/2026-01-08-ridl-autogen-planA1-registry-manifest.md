# 计划：方案A1 - registry/build.rs 生成 RIDL manifest（OUT_DIR）并通过环境变量供 mquickjs-rs/build.rs 消费

## 背景
我们希望新增 RIDL module 时只需要在 `ridl-modules/registry/Cargo.toml` 添加一个 path 依赖（即“单一注册点”），无需修改 registry 的 Rust 代码。
同时，`mquickjs_ridl_register.h` 仍由 `mquickjs-rs` 在 build.rs 阶段调用 ridl-tool 生成（职责：mquickjs 标准库构建）。

## 目标
- `ridl-modules/registry/build.rs` 自动生成 RIDL 清单：`$OUT_DIR/ridl_manifest.json`
- 并通过 `cargo:rustc-env=RIDL_REGISTRY_MANIFEST=...` 将清单路径注入后续构建步骤
- `deps/mquickjs-rs/build.rs` 读取 `RIDL_REGISTRY_MANIFEST`，基于清单执行：
  - per-module 生成（glue/impl）
  - aggregate 生成（`mquickjs_ridl_register.h`、`ridl_symbols.rs` 等）

## 重要约束/规则
- **RIDL module 判定规则**：仅当 registry 的某个 path 依赖满足 `<dep_path>/src/*.ridl` 至少存在 1 个时，才认为是 RIDL module；否则排除。
- 不稳定路径问题：**不写入 workspace 根**，只写入 registry 的 OUT_DIR；跨项目可复用。

## 设计
### registry/build.rs
1) 解析 `ridl-modules/registry/Cargo.toml`
   - 只看 `[dependencies]` 下包含 `path = "..."` 的条目
2) 对每个 path 依赖：
   - 计算绝对路径 `dep_dir`
   - 判断 `dep_dir/src/*.ridl` 是否存在
     - 若不存在：跳过（不是 ridl module）
     - 若存在：收集所有 `.ridl`（稳定排序）
3) 生成 `ridl_manifest.json`
   - 内容：JSON 数组，元素为 ridl 的**绝对路径字符串**（便于 mquickjs-rs/build.rs 直接消费）
4) 输出 cargo 指令：
   - `cargo:rerun-if-changed=Cargo.toml`
   - 对每个 ridl 文件输出 `cargo:rerun-if-changed=<abs>`
   - `cargo:rustc-env=RIDL_REGISTRY_MANIFEST=<out_dir>/ridl_manifest.json`

### mquickjs-rs/build.rs
1) 优先读取环境变量 `RIDL_REGISTRY_MANIFEST`
2) 若存在：
   - 解析 JSON，得到 ridl 绝对路径列表
   - 以列表为输入执行现有 module/aggregate 生成逻辑
3) 若不存在：
   - 回退到旧的目录扫描（兼容性），或直接报错（待定，建议先兼容）
4) `cargo:rerun-if-env-changed=RIDL_REGISTRY_MANIFEST`

## 测试策略
- 新增/扩展单元/集成测试，验证：
  - 在 registry 只通过 Cargo.toml 添加 path 依赖时，manifest 能列出对应 ridl
  - mquickjs-rs 构建时生成的 `deps/mquickjs-rs/generated/mquickjs_ridl_register.h` 包含来自清单的符号（如 `js_sayhello`）
  - 过滤规则生效：对没有 `src/*.ridl` 的 path 依赖不会进入 manifest

## 验收标准
- 新增模块时：只修改 `ridl-modules/registry/Cargo.toml`（加 path 依赖）即可被纳入聚合。
- `mquickjs_ridl_register.h` 的内容与 manifest 对应（包含所有 RIDL 模块提供的 C 接口）。
- 构建/测试通过。
