<!--
status: 已归档（实现路径变更：未采用“直接 return Local<any>”；改为 ReturnAny + pin_return）
owner: Mi Code
tags: ridl, glue, any, api, design
-->

# 2026-01-24：去掉 `*_out` 机制，让用户层“直接返回 any”的可行性分析

> 结论先行（草案）：
>
> - `*_out` 并非纯 glue 内部细节，它已经进入 **用户实现 trait** 的 API（见生成的 `api.rs`）。
> - 要“去掉 `*_out` 并让用户层直接返回 any”，我们必须为 `any` 返回值定义一个 **可安全跨 glue 边界的 Rust 类型**，并解决 **HandleScope/逃逸（escape）/生命周期** 的约束。
> - 在现有 mquickjs-rs 的 V8-style handle 体系下，可行方向是：
>   - 用户返回 **可被 glue 安全接管并 root 的值**（例如 `mquickjs_ffi::JSValue` 或一个封装的新类型），而不是返回带生命周期的 `Local<'ctx, Value>`。
>   - 或者用户返回一个“在 env/scope 内构造并 escape”的值（需要新的 API 形态）。

## 0. 现状与证据

### 0.1 当前代码里：any 的 Rust 边界映射是 `Local<Value>`，但 any-return 被强制走 `*_out`

#### 0.1.1 any 的“类型映射”（generator filters）

在 `deps/ridl-tool/src/generator/filters.rs`：

- `Type::Any` 的 `rust_type_from_idl` =
  - `mquickjs_rs::handles::local::Local<'_, mquickjs_rs::handles::local::Value>`

同时 `emit_return_convert_typed` 对 `Type::Any` 的返回转换是：

- `result.as_raw()`（也就是：**如果我们能拿到一个 `Local<Value>` 作为 result，glue 可以直接返回 JSValue**）

#### 0.1.2 但 trait 声明模板（rust_api.rs.j2）对 any-return 做了 ABI 特判

生成模板：`deps/ridl-tool/templates/rust_api.rs.j2`

- 当 `method.return_type == Type::Any` 时：
  - 生成 `fn xxx(...) -> ();`（并在测试实现里 `unreachable!("any-return must use xxx_out")`）
  - 同时生成 `fn xxx_out<'ctx>(..., out: &mut dyn for<'hs> FnMut(Any<'hs,'ctx>), ...) -> ();`

也就是说：当前实现**不是因为无法“把 any-return 转成 JSValue”**，而是因为 ABI/签名层面刻意把 any-return 从 `Local<Value>` 改成了 out-callback。

#### 0.1.3 glue 模板目前只为 singleton 实现了 any-return 的 out 分支

模板：`deps/ridl-tool/templates/rust_glue.rs.j2`

- singleton method 返回 any 的实现：
  - glue 在栈上保存 `Option<JSValue>`
  - 调用 `singleton.xxx_out(&mut |v| __ridl_any_out = Some(v.as_raw()))`
  - 要求 out 恰好调用一次，否则抛异常

这说明当前 `*_out` 的实际目的更具体：
- 让“构造返回 any”的过程发生在一个具有 `'hs` 生命周期的回调里，从而能使用 `Any<'hs,'ctx>`（rooted handle），
- glue 拿到 raw JSValue 后返回。

### 0.2 历史设计口径（文档）与当前代码口径存在分歧

- 2026-01-18 的测试扩展计划里曾写：
  - “Rust 边界 any 映射为 JSValue（避免生命周期不可表达）”
- 但当前实际代码：
  - any param 映射为 `Local<'ctx, Value>`（不是 raw JSValue）
  - any return 仍被强制走 `*_out`，并在 out 中使用 `Any<'hs,'ctx>`

结论：要讨论“去掉 *_out”，必须先统一我们到底希望：
- any 的 Rust 边界类型是 `Local<Value>`（view）
- 还是 raw `JSValue`
- 或者某种 owned wrapper

本分析后续将以“当前代码已经选了 any param = Local<Value>”为基线。

## 1. 为什么 `return any` 会引出 `*_out`

### 1.1 必须先澄清：本仓库的引擎（mquickjs）与 QuickJS 的 JSValue 语义不同

- QuickJS 官方手册说明：JSValue 是引用计数，C 函数返回“新分配 (=live) 的 JSValue”，需要 JS_DupValue/JS_FreeValue 管理。
- 但本仓库的引擎是 **mquickjs fork**，其 README 明确写到：
  - “it relies on a tracing garbage collector”
  - “the VM does not use the CPU stack”

这意味着我们不能直接照搬 QuickJS 的“返回 newly allocated JSValue”的内存语义；我们必须以 mquickjs-rs 的 handle/root 机制为准。

### 1.2 mquickjs-rs 的三层值语义（Local / Handle / Global）

在 mquickjs-rs 里（代码注释也写得很明确）：

- `Local<'ctx, T>`：绑定 context 的 view，**不是 GC root**。
- `Handle<'hs,'ctx,T>`：由 `HandleScope` root 的 handle，生命周期绑定到 scope。
- `Global<T>`：持久 root。

证据：`deps/mquickjs-rs/src/handles/handle_scope.rs` / `handles/global.rs`。

### 1.3 `*_out` 的真实用途（结合现状重新解释）

结合 0.1 的事实：
- generator 已经能把 `Local<Value>` 转成 JSValue（`emit_return_convert_typed(Type::Any) => result.as_raw()`）
- 但模板仍把 any-return 改成 `*_out` + `Any<'hs,'ctx>`

因此 `*_out` 不是为了解决“无法把 any-return 变成 JSValue”，而是为了确保：

- 用户在 Rust 实现里构造的返回值，处于一个 **被 HandleScope root 的生命周期 `'hs`** 内（通过 `out: for<'hs> FnMut(Any<'hs,'ctx>)` 表达），
- glue 拿到 `Any` 后在 scope drop 前把 raw JSValue 返回。

换句话说：`*_out` 是用来强制 any-return 走“rooted handle”路径，从而避免用户返回一个没有被 root 的 `Local<Value>`。

### 1.4 因此，“去掉 *_out”要回答的关键问题

如果我们让用户直接 `-> Local<'ctx, Value>`：

- glue 是否需要（以及是否能够）在返回前把该 Local 临时 root？
- 或者 mquickjs 对 native return value 是否天然视为 GC root（至少直到调用方把它放入对象/栈）？

在 mquickjs-tracing GC 语义下，**这点必须用 mquickjs 的实现/实验来确认**，不能只靠 QuickJS 文档推断。

下面的方案章节会把“验证实验”作为前置步骤：先用最小 C/Rust glue 试验在返回后立刻触发 GC，再读取返回值，确认是否会悬挂。

## 2. 去掉 `*_out` 的目标澄清

用户提议：“去掉 `*_out`，用户层直接返回 any”。这里的“直接返回 any”有三种可能解释：

1) 返回 `Local<'ctx, Value>`（带生命周期）
2) 返回 `Any<'hs,'ctx>`（带 scope 生命周期）
3) 返回“无生命周期”的可拥有值（owned/raw），由 glue 接管

其中 (1)(2) 都把生命周期暴露给用户；(3) 则把生命周期隐藏掉，但要求我们定义一个 owned 载体。

如果我们希望 **用户实现层体验最简单**，通常倾向 (3)：

- `fn f(&mut self, env: &mut Env<'_>, ...) -> AnyOwned`（示例名）
- 或 `-> mquickjs_ffi::JSValue`（raw，风险更大）

## 3. 可行方案对比

### 方案 A：用户返回 raw `mquickjs_ffi::JSValue`

**API 形态（示意）**：

```rust
fn f(&mut self, env: &mut Env<'_>, ...) -> mquickjs_rs::mquickjs_ffi::JSValue;
```

**优点**：
- 最直接，不需要 `*_out`。
- glue 直接把 `JSValue` 返回给 QuickJS。

**风险/缺点**：
- raw `JSValue` 太底层：
  - 容易出现 cross-context 值
  - 容易返回未被 root 的值（取决于 QuickJS 对返回值的处理）
  - 更容易写出 UB 风格的 FFI 错误
- 与仓库规则冲突风险：项目强调不要手动 free/dup JSValue，但 raw 值会诱导错误用法。

**适用性**：仅适合作为内部 glue 层，不建议作为用户层 trait API。

### 方案 B：引入 `AnyOwned`（或类似）作为“可返回 any 的拥有类型”（推荐候选）

**补充现状（重要）**：mquickjs-rs 目前已经有两类相关类型：

- `Local<'ctx, Value>`：仅绑定 context，不是 GC root（见 `handles/local.rs` 注释与实现）。
- `Any<'hs,'ctx>`：内部其实是 `Handle<'hs,'ctx,Value>`，即 **已被当前 HandleScope root** 的值（见 `handles/any.rs`）。
- 另外还有 `Global<T>`：通过 `JS_AddGCRef` 形成跨 scope 的 GC root（见 `handles/global.rs`）。

因此，要“去掉 `*_out`”而又不把 `'hs` 生命周期暴露给用户，我们需要一个 **owned 的 any 返回类型**。

**核心思想**：
- 用户返回一个“可拥有”的 any 值（内部持有 raw `JSValue` + `ContextId`）。
- glue 收到后：
  1) 校验 ctx 一致（防 cross-context）
  2) 将 raw 值挂到当前 `HandleScope`（`push_gc_ref`）或其它等价 root 机制
  3) 把 raw `JSValue` 返回给 QuickJS

**API 形态（示意）**：

```rust
fn f(&mut self, env: &mut Env<'_>, ...) -> mquickjs_rs::AnyOwned;
```

**实现 AnyOwned 的两种子路径**：

- B1：`AnyOwned` = `Global<Value>`
  - 用户通过 `Global::new(scope, local)` 构造
  - glue 将 `Global::as_raw()` 直接返回
  - 缺点：Global 生命周期跨调用，需要 Drop 时机；作为“返回值”会把所有权语义搞复杂（返回后由谁 drop？）

- B2：`AnyOwned` = `OwnedValue { raw, ctx_id }`（不自动 root）
  - 用户通过 env 提供的工厂方法构造（例如 `env.owned(local)` / `env.owned_raw(raw)`）
  - glue 在返回前把它 root 到当前 `HandleScope`（仅为本次返回过程确保安全）
  - 关键问题：我们需要确认 QuickJS 对 native 返回值的可达性规则；若返回值自动变为可达，则这里的 root 可能只是“保守兜底”。

**需要新增/确认的 mquickjs-rs API**：
- `AnyOwned` 类型本身（至少 `as_raw()` + `ctx_id()`）。
- `Env` 或 `HandleScope` 侧提供一个“把 raw/owned 值纳入当前 scope GCRef 列表”的公开方法（目前 `push_gc_ref` 是 crate 私有）。

**优点**：
- 用户层不需要理解 `*_out`，也不需要面对 `for<'hs>` 回调。
- 类型系统可做 ctx 校验，减少 UB。

**缺点/风险**：
- 仍需在 mquickjs-rs 增加 API（尤其是“root 到当前 scope”的公开能力）。
- 需要明确 AnyOwned 的“所有权/Drop 语义”以避免泄漏或二次 drop。

### 方案 C：用户返回 `Handle<'hs,'ctx, Value>`（由 glue 提供 scope）

**形态（示意）**：

```rust
fn f<'hs, 'ctx>(&mut self, env: &mut Env<'ctx>, ...) -> Handle<'hs, 'ctx, Value>;
```

这基本不可行：
- trait 方法无法在返回类型上携带由调用方决定的 `'hs`（除非引入 HRTB/复杂泛型），且实现者体验很差。

结论：不推荐。

## 4. 推荐方向（修订：以“直返 Local”作为第一优先方案）

基于最新调研，推荐按以下优先级推进：

### 4.1 方案 D（第一优先）：any-return 直返 `Local<'ctx, Value>`，并**删除 `*_out`**

理由：
- **当前 generator 的类型映射已经是 Local**（0.1.1），并且返回转换已支持 `result.as_raw()`。
- `*_out` 是 ABI 特判引入的额外复杂度；并且 glue 目前只实现了 singleton 的 out 分支，导致 class/module 的 any-return 不完整。
- `Local<'ctx, Value>` 不携带 `'hs`，因此 trait 依然对象安全（`dyn Trait` 不受影响）。

该方案的关键风险只有一个：
- mquickjs tracing GC 下，native 返回的 raw JSValue 是否天然可达（无需我们额外 root）。

因此需要把“验证实验”作为前置门槛：

- D0：构造一个函数 `fn retObject() -> any`（实现返回一个新建 object/string/array）
- D1：在 JS 侧立刻施加 GC 压力（分配大量对象/数组），并读取返回值的属性/内容
- D2：若稳定通过，说明“返回值自动成为可达值”，可以安全采用直返 Local。

### 4.2 方案 B（备选）：引入 owned 返回类型（AnyOwned）

如果 D0~D2 失败（返回值在 JS 侧很快变悬挂或崩溃），则必须提供一个“显式 root”的返回载体。
这时再回退到方案 B。

## 5. 实施计划（以方案 D 为主）

### 5.1 Phase 0：验证实验（必须先做）

- 在 tests 目录新增一个最小模块/用例：
  - `ret_obj() -> any`：Rust 侧通过 env/scope 构造 object/string/array，返回 `Local<Value>`
  - JS 用例：`let o = t.ret_obj();` 后立即进行大量分配，再断言 `o.k === 1` 或 `o === o`。

> 该实验的目标不是覆盖，而是确定 GC 语义：返回值是否自动可达。

### 5.2 Phase 1：调整 rust_api.rs.j2（移除 any-return 的 *_out 特判）

- 对 `return_type == Type::Any`：
  - 直接生成 `) -> {{ method.return_rust_ty }}`
  - 删除 `xxx_out` 声明

### 5.3 Phase 2：调整 rust_glue.rs.j2（删除 singleton any-return 的 out 分支）

- 移除 `__ridl_any_out` 逻辑
- 统一走：
  - `let result = singleton.xxx(...);`
  - `emit_return_convert_typed(Type::Any, "result")` => `result.as_raw()`

### 5.4 Phase 3：迁移 tests/ 下所有 any-return 实现

- 将所有 `*_out` 实现改为直接返回 `Local<'ctx, Value>`
- 删除 unreachable 的 stub 方法

### 5.5 Phase 4：补齐 class/module/singleton any-return 的 identity 用例

- singleton：`echoAny(v:any)->any` 断言 `===`
- module fn：同上
- class method：同上

### 5.6 验证

- `cargo run -p ridl-builder -- prepare`
- `cargo test`
- `cargo run -- tests`

## 6. 兼容性与迁移评估

- 这是一个 **breaking change**：用户实现 trait 的签名会变（删除 `*_out`）。
- 但在本 demo 仓库里，主要影响集中在 `tests/` 目录的实现，迁移成本可控。

## 5. 需要修改的代码范围（预估）

### 5.1 ridl-tool

- `rust_api.rs.j2`：
  - 删除 `return any` 生成 `*_out` 的分支
  - 改为 `-> <AnyOwnedType>`
- `rust_glue.rs.j2`：
  - 删除 `*_out` 调用逻辑
  - 统一成 `let result = xxx(...);` 再转换为 `JSValue`

### 5.2 mquickjs-rs

- 新增/调整一个“owned any value”类型与转换 API（具体命名需与现有风格一致）。
- 提供 glue 可用的“root 该值到当前 scope”能力。

### 5.3 tests

- 更新所有 any-return 的测试模块实现（目前很多用例是通过 `*_out` 实现）。
- 补齐 class/module/singleton 的 identity 测试。

## 6. 迁移与兼容策略

两种路线：

### 路线 1：一次性切换（breaking change）

- 直接改模板，所有实现者同步改。
- 适合当前 demo 仓库（用户实现较少）。

### 路线 2：双栈过渡（同时支持旧 *_out 与新 return-any）

- 模板生成同时接受：
  - 优先调用新方法（直返）
  - 若未实现则 fallback 到 `*_out`

Rust trait 很难“可选实现”；需要通过：
- 生成不同 trait 名称（V2），或者
- 生成一个默认实现，内部调用另一个方法（需要稳定的默认方法 + 不破坏对象安全）

考虑到现有大量 trait 是 `dyn Trait`（对象安全关键），双栈过渡可能复杂。

建议：在本仓库先采用 **路线 1**，并在文档中标注 breaking。

## 7. 测试计划（迁移后必须新增/保持）

- `cargo run -p ridl-builder -- prepare`
- `cargo test`
- `cargo run -- tests`

测试断言重点：
- any-return identity：返回对象 `===` 输入对象
- any-return for primitive：返回 `undefined/null/number/string` 等不崩溃
- GC 稳定性：返回对象在函数返回后仍可被访问（可加一次 `gc()` 或触发分配）

## 8. 仍需进一步确认的问题清单

1) mquickjs-rs 是否已经有“owned JSValue”类型（例如 `Value`/`Any` 的 owned 形态）？如果有，优先复用。
2) QuickJS 对 native function 返回 `JSValue` 的 rooting 规则：
   - 返回值是否自动成为可达对象？
   - 在我们 wrapper 下是否需要显式 push_gc_ref？
3) 现有 tests/global 的 any-return 用例都采用 `*_out`，迁移成本与风险评估。

---

> 状态：草案。下一步建议：先在代码中确认 mquickjs-rs 是否已有可用于“直返 any”的 owned 载体；若缺失，再设计并补齐最小 API。