# 计划：mquickjs-rs Env + Any(newtype) + Array(no-holes) + RIDL(any) 调用链（2026-01-18）

> 状态：草案（待确认）

## 0. 背景

当前 mquickjs-rs 的对外 API 以 `Context + Scope + Local/Global + HandleScope/Handle` 为主。

问题：
- 高频路径需要同时传 `&Context` 和 `&Scope`（例如 `ctx.create_string(&scope, ...)`），调用噪音大。
- 在 RIDL 的 Rust impl 场景中（尤其 `any`），这会放大为：签名臃肿 + 需要到处手动 pin/escape，影响可用性。
- 同时我们还要支持“非 RIDL 用户直接写 Rust 调 JS 引擎”的 native 开发体验。

本计划目标是在不破坏底层语义的前提下：
- 提供一个一等的执行上下文入口 `Env`，让日常操作只需要一个参数。
- 引入 `Any` 作为 `Value` 的语义特化（newtype），并补齐 `Array/Object/Function` 等必要类型。
- 明确并打通 RIDL(any) 从 JS 调入到 Rust impl 的完整调用链设计。

## 1. 关键语义约束

### 1.1 GC 与句柄层级

沿用既有层级：
- `Local<'ctx, T>`：值视图（非 GC root）
- `Handle<'hs, 'ctx, T>`：临时 GC root（由 `HandleScope` 管理）
- `Global<T>`：持久 GC root

### 1.2 mquickjs 的 Array 语义（no-holes）

来自 `deps/mquickjs/README.md`（Stricter mode / JS subset）：
- Array **不能有空洞**。
- 越界写入（在尾部之后的非相邻位置写）是 TypeError：
  - `a[0]=1` 允许（扩到末尾）
  - `a[10]=2` TypeError
- `new Array(len)` 允许，元素初始化为 `undefined`（dense，不是 hole）。
- `[1, , 3]` 语法错误。

对 Rust API 的要求：
- 不在 Rust 侧“虚构 hole”的抽象。
- 对 `index > len` 的 set：采用 **策略 A**（用户已确认）：
  - 直接调用引擎写入，让引擎抛 TypeError；Rust 包装为 Err，不提前拦截，从而保持错误语义/信息与引擎一致。

## 2. 设计概览：三层 API

### 2.1 Layer A：底层边界（保持）
- `Context` / `ContextToken` / `Scope`
- `Local` / `Handle` / `Global`
- `HandleScope` / `EscapableHandleScope`（保留；escapable 属于高级用法，不是常规返回必经）

### 2.2 Layer B：一等入口 Env（新增）

新增：

```rust
pub struct Env<'ctx, 'hs> {
    scope: &'ctx Scope<'ctx>,
    hs: &'hs mut HandleScope<'ctx>,
}

impl<'ctx, 'hs> Env<'ctx, 'hs> {
    pub fn new(scope: &'ctx Scope<'ctx>, hs: &'hs mut HandleScope<'ctx>) -> Self;

    pub fn scope(&self) -> &'ctx Scope<'ctx>;
    pub fn handle_scope(&mut self) -> &mut HandleScope<'ctx>;
}
```

目标：
- native 开发：用户只需在入口处创建 scope/hs/env，后续都通过 `&mut Env` 操作。
- RIDL：glue 每次回调自动创建 env 并传给 impl；impl 签名收敛。

### 2.3 Layer C：持久化/跨边界
- `Global<T>` 继续作为跨调用/长期保存的唯一显式机制。

## 3. 类型体系：Value / Any / Object / Array / Function

### 3.1 Value（根）
保持 `Value` 作为“任意 JS 值”的根类型标记。

### 3.2 Any（Value 的语义特化，newtype）

新增：

```rust
pub struct Any<'hs, 'ctx>(Handle<'hs, 'ctx, Value>);
```

动机：
- `Any` 不引入新的引擎类型，只表达“业务层动态值”的意图。
- RIDL/native 都常用：动态分发/打印/透传。

提供接口（示例）：
- `Any::as_value(&self) -> Handle<'hs,'ctx,Value>`
- `Any::as_local(&self, env: &Env) -> Local<'ctx,Value>`
- `Any::is_null(&self, env: &Env) -> bool` 等
- `Any::try_into_object(self, env: &Env) -> Result<Handle<'hs,'ctx,Object>, String>`
- `Any::try_into_array(self, env: &Env) -> Result<Handle<'hs,'ctx,Array>, String>`

### 3.3 Object / Function（补齐 Env 入口，复用既有 Local API）
- 已存在 `Local<Value>::try_into_object` 与 `Local<Object>::get_property/set_property`。
- 计划新增/补齐：
  - `Env::obj() -> Result<Handle<'hs,'ctx,Object>, String>`
  - `Handle<Object>` 上的便捷方法（或通过 `as_local` 复用 Local API）

### 3.4 Array（新增类型 + no-holes API）

新增类型标记：`Array`。

新增 API：

- 构造：
  - `Env::array() -> Result<Handle<Array>, String>`（len=0）
  - `Env::array_with_len(len: u32) -> Result<Handle<Array>, String>`（dense undefined 初始化）

- 基础操作：
  - `Array::len(&self, env: &mut Env) -> Result<u32, String>`
  - `Array::get(&self, env: &mut Env, index: u32) -> Result<Any, String>`（或 `Handle<Value>`）
  - `Array::set(&self, env: &mut Env, index: u32, value: Any/Handle<Value>) -> Result<(), String>`

- 便捷操作（用户确认 B）：
  - `push/pop`
  - `shift/unshift`（可选，若实现成本低）
  - `iter`（只迭代 0..len，保证 dense 语义，不暴露 hole）

no-holes 的行为定义：
- `set(i, ...)`：
  - `i <= len`：允许（i==len 等价 push）
  - `i > len`：直接调用引擎写入；引擎将 TypeError；Rust 返回 Err。

实现优先级：
1) `len/get/set/push/pop`（最小闭环）
2) `shift/unshift`（如果需要）
3) `iter`（只读便利）

## 4. Env 上的“常用构造/转换” API（简化传参）

目标：减少 `&Context + &Scope` 双参数。

示例：
- `Env::str(&mut self, s: &str) -> Result<Any, String>` 或 `Result<Handle<Value>, String>`
- `Env::undefined()/null()/bool()/int()/float()`
- `Env::get_string(&self, v: impl Into<Local<'ctx, Value>>) -> Result<String, String>`

实现原则：
- 只要能从 `Scope` 拿到 `JSContext*` 与必要的 inner，优先挂在 `Env`（或 `Scope`）上。
- 老的 `Context::create_string(&Scope, ...)` 等可保留一段迁移期，内部转调 `Env/Scope`。

## 5. RIDL(any) 调用链设计

### 5.1 glue 入口统一创建 Env
每次 JS -> Rust 回调：
1) 从 `JSContext*` 恢复 `ContextToken`
2) `enter_scope()`
3) 创建 `HandleScope`
4) `Env::new(&scope, &mut hs)`

### 5.2 impl 签名收敛
RIDL 生成的 Rust impl 统一把 `&mut Env` 作为第一参数：

```rust
fn foo(env: &mut Env, a: Any, b: i32) -> Result<Any, Error>
```

参数类型策略：
- RIDL 的 `any` 映射为 `Any`。
- 其他对象类型（如 `Object`/`Array`）可逐步映射为强类型 handle。

返回值策略：
- impl 返回 `Any`（内部是 handle，GC safe）；glue 在 handle-scope drop 前把 raw `JSValue` 直接返回给引擎。
- 典型返回不要求用户显式使用 `EscapableHandleScope`。

## 6. 迁移策略

### 6.1 API 兼容
- 逐步将高频 API 从 `Context` 下沉到 `Env`（或 `Scope`）。
- `Context::xxx(&Scope, ...)` 在迁移期保留，内部薄封装调用 `Env/Scope`。

### 6.2 内部实现复用
- `Local<Object>` 等现有实现尽量复用。
- `Handle<T>` 提供 `as_raw()`；通过 `env.scope().value(...)` 生成 `Local` 复用 Local API。

## 7. 测试矩阵（实现前先写）

### 7.1 Rust 单元测试（mquickjs-rs）
- `env_create_string_survives_gc`：`env.str` 创建的值通过 handle-scope pin，GC 后可取回。
- `array_set_out_of_bounds_is_typeerror`：
  - 创建 `arr=[]; arr.push(1)`
  - 调用 Rust `Array::set(index=len+9, ...)`，必须返回 Err（由引擎 TypeError 导致）。
- `array_set_at_end_extends`：`set(len, v)` 等价 push，len 增加。
- `array_with_len_is_dense_undefined`：`new Array(len)` 行为等价：元素为 undefined，可 get。

### 7.2 trybuild
- 固化 `Any/Handle` 不能跨 env 生命周期逃逸（根据最终 API 细节补用例）。

### 7.3 repo 根 JS 集成测试
- 增加一条用例：JS 调用 RIDL 暴露的函数，传入 any，Rust 侧构造/返回 Array，验证：
  - push/pop 正常
  - 越界 set 触发 TypeError（JS 侧捕获）

## 8. 待确认点

1) Env 的模块位置与 re-export：建议在 `mquickjs-rs` crate 根导出 `Env`。
2) Any/Array 的返回类型：`Any` 是否统一使用 `Handle<Value>` 作为内部承载（本计划默认是）。
3) Array API：是否需要 `shift/unshift`（计划可选项，默认后置）。

---

> 如果你确认本计划，我将按本仓库流程：先补齐/对齐 API.md 文档，再落地实现 + 测试 + 跑 `cargo test` 与 `cargo run -- tests`。
