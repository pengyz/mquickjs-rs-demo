<!-- planning-meta
status: 未复核
tags: context-init, engine, ridl, tests, types
replaced_by:
- docs/ridl/overview.md
- docs/ridl/context-init.md
-->

> 状态：**未复核**（`context-init` `engine` `ridl` `tests` `types`）
>
> 现行口径/替代：
> docs/ridl/overview.md
> docs/ridl/context-init.md
>
> 关键结论：
> - （待补充：3~5 条）
# 目标

修复 workspace 中的编译警告，优先处理 **生成的胶水代码（OUT_DIR/*_glue.rs、*_symbols.rs、ridl_context_init.rs 等）** 引入的 warning，保持 generator 可维护性，不引入任何“模块白名单/硬编码 singleton 名称”的简化逻辑。

> 约束：C API 注册必须编译期完成；生成聚合/注册逻辑必须通用可扩展（新增模块仅放入 ridl-modules/ 生效）。

# 现状盘点（来自 `cargo test -q` 输出）

## 非生成代码（手写）
- `deps/mquickjs-rs/src/context.rs`:
  - unused variable: `inner`
  - dead_code: `Context.inner` never read
- `src/test_runner.rs`:
  - unused import: `crate::Context`
- `ridl-modules/stdlib/src/../stdlib_impl.rs`:
  - unused import: `crate::generated::api::ConsoleSingleton`

这些属于“项目代码层面”的 warning；是否一并修由需求决定。

## 生成代码（重点）
路径集中在：
- `target/**/out/*_glue.rs`
- `target/**/out/*_symbols.rs`
- `target/**/out/ridl_context_init.rs`

常见 warning 类型：
- `unused_unsafe`: `unsafe { JS_IsString(ctx, unsafe { *argv.add(0) }) }` 这类 **嵌套 unsafe**
- `unused_variables`: `let rel = i - 0;` 在某些 varargs 分支中不使用
- `unused_imports`: 例如 `use std::os::raw::{c_char, c_int};` 中 `c_char` 未使用；或 symbols.rs 中 `JSContext/JSValue` 未使用
- `unused_mut`: `let Some(mut h) = ...` 中 `mut` 不需要
- `dead_code`: `const JS_UNDEFINED` 未使用（当返回值分支不需要时）

# 策略

## A. 优先修 generator 输出（本次主要范围）
1. **消除嵌套 unsafe**：
   - 约定：生成代码尽量只在需要的地方使用一次 `unsafe`，避免 `unsafe { ... unsafe { ... } ... }`。
   - 做法：在生成的 glue 中先 `let v = unsafe { *argv.add(idx) };`，后续 `unsafe { JS_IsX(ctx, v) }`。
   - 对固定参数与 varargs 都适用。

2. **仅在需要时生成 `rel`**：
   - `rel` 只在错误信息中用到（`...rest[{rel}]`），Any varargs 不需要错误信息时不生成 rel。
   - 或者统一生成 `let _rel = ...`（但更偏“压警告”，不如按需生成）。

3. **按需导入/定义**：
   - `c_char`：仅 string/cstring 路径需要时导入。
   - `JS_UNDEFINED`：只在确实需要返回 undefined 的函数生成该 const；否则直接返回 `mquickjs_rs::mquickjs_ffi::JS_UNDEFINED`。
   - `*_symbols.rs`：如果只是为了 keep-alive，可避免未使用 import（改为引用一次或使用 `#[allow(unused_imports)]` 仅限生成文件）。

4. **避免无意义的 `mut`**：
   - `let Some(h) = ...`，除非后续确实 `&mut` 绑定需要 `mut`。

## B. 手写代码 warning（纳入本次范围）
这部分 warning 不是“生成胶水”引入的，但你已确认希望一并清掉。

# 验收
- `cargo test -q` 输出中：生成文件相关 warning 数量显著下降，优先清零 `*_glue.rs` 的 `unused_unsafe/unused_variables/unused_mut/unused_imports/dead_code`。
- `cargo run -q -- tests` 仍通过。

# 实施步骤
1. 调整 ridl-tool 的 glue generation（filters.rs + rust_glue.rs.j2）：
   - 固定参数提取路径：先绑定 `v`，避免嵌套 unsafe。
   - varargs：仅在需要时生成 `rel`；Any 分支不生成 rel。
   - return undefined：直接使用常量而非引入未用 const。
   - imports：按需生成。
2. 调整 symbols/context init 生成逻辑以避免 unused imports。
3. 更新/新增测试：对生成输出做 contains 断言，确保不再出现 `unsafe { ... unsafe {` / `let rel =`（Any）/ `use ... c_char`（无 string）等模式。
4. 全量回归。

# 状态
- [ ] 进行中
