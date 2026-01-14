# RIDL 模块级生成文件收敛方案（每模块 2 文件：api.rs + glue.rs）

日期：2026-01-14

状态：草案（待审阅）

> 目标：rid l模块 crate（例如 `ridl-modules/stdlib`）当前在 `OUT_DIR` 生成多个带模块名前缀的文件（`<module>_glue.rs`、`<module>_api.rs`、`*_symbols.rs`、`ridl_module_api.rs` 等），导致：
>
> - 文件名冗余（模块名重复；OUT_DIR 天然隔离无需前缀）。
> - 用户侧 `lib.rs/generated.rs` include 多文件，难以统一。
> - symbols/module_api 等“内部 glue 细节”暴露给用户，容易误用。
>
> 本方案将模块级产物收敛为 **2 个文件**：`api.rs`（用户最小依赖）+ `glue.rs`（桥接/初始化/保活细节全部封装）。

---

## 1. 现状梳理（基于扫描）

以现有模块为例：

- `ridl_tool::generator::generate_module_files(items, mode, out_dir, module_name)` 生成：
  - `<module>_glue.rs`
  - `<module>_api.rs`

- `ridl_tool::generator::generate_module_api_file_default(out_dir)` 生成：
  - `ridl_module_api.rs`（固定名）

- 模块 `build.rs` 为了生成 `<module>_symbols.rs`，采用“借用 aggregate shared 生成”的方式：
  - `generate_shared_files([单个 ridl], out_dir)` 会写出 `ridl_symbols.rs`
  - 随后 build.rs rename 成 `<module>_symbols.rs`（例如 `stdlib_symbols.rs`）

- stdlib 还存在额外生成 `ridl_ctx_ext.rs` 的过渡逻辑（应后续清理/重新定位职责）。

模块 crate 的 `lib.rs/generated.rs` 因此需要 include 多个 OUT_DIR 文件。

---

## 2. 设计原则

1) **OUT_DIR 天然隔离**：模块名不需要出现在生成文件名里。
2) **用户代码最小依赖**：用户实现逻辑只应依赖 `api`（trait/类型），不应直接 include symbols/module_api 等底层文件。
3) **胶水自封装**：glue 应当内部完成：
   - symbols 保活（ensure_symbols）
   - module 初始化入口（initialize_module / module_context_init）
   - JS->Rust 绑定桥接（js_* entrypoints）
4) **消费侧统一**：提供一个宏让模块 `lib.rs` 只写一行 include。

---

## 3. 收敛目标（模块级最终产物）

每个 RIDL module crate 的 `OUT_DIR` 最终只生成：

- `api.rs`
  - 内容：trait/类型/常量声明（供用户 impl 引用）
  - 不生成 `todo!()` stub
  - 不依赖 app crate 类型

- `glue.rs`
  - 内容：
    - JS glue entrypoints（原 `<module>_glue.rs`）
    - module initializer API（原 `ridl_module_api.rs` 的函数，但不再单独生成文件）
    - symbols 保活逻辑（原 `<module>_symbols.rs` 的 ensure_symbols 逻辑，合入 glue 内部模块）
  - 对外暴露：
    - `pub fn initialize_module()`
    - `pub fn ridl_module_context_init(w: &mut dyn mquickjs_rs::ridl_runtime::RidlSlotWriter)`
    - （以及原 glue 中需要导出的 js_* symbols，如果模块需要 re-export）

> 说明：本方案刻意不在模块 OUT_DIR 生成单独 `symbols.rs` / `module_api.rs` 文件，避免用户误 include。

---

## 4. mquickjs-rs 提供统一 include 宏（你已确认）

在 `deps/mquickjs-rs` 新增宏：

- `mquickjs_rs::ridl_include_glue!()`

展开为：

```rust
include!(concat!(env!("OUT_DIR"), "/glue.rs"));
```

可选增强（按需决定是否提供）：

- `mquickjs_rs::ridl_include_api!()`（include `api.rs`）

但考虑到 `api` 更适合以 `mod api { include!(...) }` 组织，宏不是必须。

模块 crate 推荐组织方式：

```rust
pub mod api {
    include!(concat!(env!("OUT_DIR"), "/api.rs"));
}

mquickjs_rs::ridl_include_glue!();

pub mod impls; // 用户实现（只依赖 api）
```

---

## 5. ridl-tool 改造点（关键）

### 5.1 新的模块级生成入口

提供一个新的 generator API（命名待定，但需表达“模块级两文件输出”）：

- 输入：`items`, `file_mode`, `out_dir`
- 输出：
  - 写 `api.rs`
  - 写 `glue.rs`

其中：

- 现有 `rust_api.rs.j2` 可复用（但输出文件名改为 `api.rs`）。
- 现有 `rust_glue.rs.j2` 可复用（但输出文件名改为 `glue.rs`），并在模板内：
  - 内嵌/合并 `ridl_module_api.rs` 的默认实现
  - 内嵌/合并 symbols ensure 逻辑（从现有 `symbols.rs.j2` 抽取，但不再生成独立文件）

### 5.2 删除/停止旧路径

- 停止生成 `<module>_glue.rs` / `<module>_api.rs`（不再带模块名）。
- 不再依赖模块 `build.rs` 通过 `generate_shared_files` 生成 `ridl_symbols.rs` 再 rename。
  - symbols 逻辑直接由模块级新模板在 `glue.rs` 内生成。

### 5.3 用户最小依赖保证

- `api.rs` 不引用 `impls`。
- `glue.rs` 引用 `crate::impls`（用户实现），并引用 `crate::api`（接口）。
- `glue.rs` 内的 symbols ensure 逻辑：
  - 不 `use crate::generated::glue::*` 导致重复定义
  - 建议采用 `extern "C" { fn js_xxx(...) }` + 取地址的方式（与 aggregated_symbols.rs.j2 同构），确保最稳。

---

## 6. 模块 crate 的 build.rs 收敛

模块 build.rs 改为：

- 只解析/校验 ridl
- 调用 ridl-tool 新入口生成 `api.rs` + `glue.rs`
- 不再：
  - 调用 `generate_shared_files`
  - rename `ridl_symbols.rs`
  - 生成 `ridl_module_api.rs`
  - stdlib 特有的 `ridl_ctx_ext.rs`（后续在“聚合文件收敛”中一并清理归位）

---

## 7. 兼容与迁移策略

按你的要求：不做薄壳兼容。

- 修改 ridl-tool + modules build.rs + modules lib.rs/generated.rs + mquickjs-rs 宏
- 全量测试通过后，旧文件名直接消失。

---

## 8. 测试与验收

- Rust：
  - `cargo test`

- JS 集成：
  - `cargo run -- tests`

- 产物验收：
  - 任一模块 crate 的 OUT_DIR 中不再出现：
    - `<module>_glue.rs` / `<module>_api.rs` / `ridl_module_api.rs` / `*_symbols.rs`
  - 仅存在：`api.rs`、`glue.rs`

---

## 9. 待你确认的关键点

1) `api.rs` 是否要求放在 `pub mod api { ... }` 中（推荐），还是直接 include 到根模块？
2) `glue.rs` 是否允许直接 `pub use` 部分 js_* glue symbols（保持现有行为）？
3) symbols ensure 逻辑是使用：
   - A) `extern` + 取地址（更稳，避免重复定义）
   - B) `use crate::generated::glue::js_xxx`（更短，但更易踩重复定义边界）

本方案推荐 1=是，3=A。
