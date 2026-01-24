<!--
status: 已完成
owner: Mi Code
tags: ridl-tool, templates, glue, scope, lifetime, any
-->

# 2026-01-24：glue 模板生成策略归档（has_scope / needs_scope / 'ctx）

> 目的：归档当前 RIDL v1 的生成策略细节，尤其是：
> - 何时生成 `Scope/Env`（has_scope / needs_scope）；
> - 何时引入 `'ctx` 泛型；
> - `any` 参数/返回与 `ReturnAny + pin_return` 的协作；
> - 如何避免 `'ctx` 泄露并保持 trait object-safe。

## 1. 术语与核心字段

### 1.1 needs_scope（方法级）

来源：`deps/ridl-tool/src/generator/mod.rs` 中 `TemplateMethod::from_with_mode`。

定义（当前实现）：
- `needs_scope = true` 当且仅当：
  - 任一参数是 `any` / `optional<any>`；或
  - variadic 参数且其元素类型是 any-like；或
  - 返回值是 `any` / `optional<any>`。

原因：
- any-like 类型在 glue/API 层需要 JS 上下文相关能力（提取、构造或返回边界 pin）。

### 1.2 has_scope（函数级 / glue 级）

在 glue 模板里通常以 `method.needs_scope`（或 function 等价逻辑）驱动：
- 若 `needs_scope=true`：生成
  - `let scope = h.enter_scope();`
  - `let mut env = mquickjs_rs::Env::new(&scope);`
- 若 `needs_scope=false`：不生成 scope/env。

> 备注：本仓库代码中有些地方叫 `needs_scope`，但讨论中也称为 `has_scope`。
> 为避免歧义，本文用：
> - `needs_scope` 指模板数据模型上的布尔值；
> - `has_scope` 指 glue 生成结果是否真的创建了 `Scope/Env`。

## 2. API trait（rust_api.rs.j2）生成规则

文件：`deps/ridl-tool/templates/rust_api.rs.j2`

### 2.1 object-safe 约束

API trait 必须保持 object-safe：
- crate 里大量使用 `Box<dyn Trait>` 作为跨边界对象（类/单例/接口）。
- 因此，不能让 trait 自身携带泛型生命周期参数（例如 `trait Foo<'ctx>`）。

当前策略：
- trait 本身不带生命周期参数。
- **方法**声明允许引入 `<'ctx>`：
  - `fn method<'ctx>(&mut self, ... ) -> ...;`

### 2.2 何时引入 env: &mut Env<'ctx>

- 若 `method.needs_scope=true`：方法签名包含
  - `env: &mut mquickjs_rs::Env<'ctx>`
- 否则：不包含 env 参数。

### 2.3 any 参数的生命周期绑定策略（关键）

问题背景：
- `&mut Env<'ctx>` 在 Rust 中对 `'ctx` **不变**（invariant）。
- 因此，在实现里调用 `env.return_safe(v)` 时，要求 `v: Local<'ctx, _>`。
- 但若方法本身不含 env（`needs_scope=false`），则无法在参数类型中引入 `'ctx`，否则会出现
  “未声明 `'ctx` 泄露”。

最终规则：
- 对每个参数 `p`：
  - 若 `method.needs_scope=true && p.ty == any`：
    - 参数类型生成：`Local<'ctx, Value>`
  - 否则：按 `p.rust_ty` 生成（对 any 来说即 `Local<'_, Value>`，来自 `rust_type_from_idl`）。

对应模板片段（概念表达）：
- `if method.needs_scope { if p.ty == Any { Local<'ctx, Value> } else { p.rust_ty } } else { p.rust_ty }`

## 3. any 返回值策略：ReturnAny + pin_return

### 3.1 为什么 any return 不用 Local<'ctx, Value>

文件：`deps/ridl-tool/src/generator/mod.rs` 中 `TemplateMethod::from_with_mode`。

约束：
- `Local<'ctx, _>` 带有调用点生命周期，会把 `'ctx` “传染”到返回类型。
- 这会让 object-safe API 变得脆弱（更容易触发 HRTB/生命周期推导复杂度），并且也不符合
  “返回跨 native->JS 边界需要 pin” 的需求。

策略：
- `return any` 映射到“可跨返回边界携带”的 owned wrapper：
  - `ReturnAny`（别名：`ReturnSafe<Value>`）

实现位置：
- `deps/ridl-tool/src/generator/mod.rs`
  - `Type::Any` => `mquickjs_rs::handles::return_safe::ReturnAny`
  - `Option<any>` => `Option<ReturnAny>`

### 3.2 glue 侧如何返回 ReturnAny

文件：`deps/ridl-tool/templates/rust_glue.rs.j2`

- 在调用用户实现后：
  - `let result = ...;`
  - 若 `return_type == any`：
    - `env.pin_return(result)`

### 3.3 Env::return_safe / pin_return（mquickjs-rs）

文件：`deps/mquickjs-rs/src/env.rs`

- `Env::return_safe(v: Local<'ctx, T>) -> ReturnSafe<T>`
  - 语义：通过 `HandleScope` 先 pin/root 一次，构造 `ReturnSafe`（保存 raw + ctx_id）。
- `Env::pin_return(v: ReturnSafe<T>) -> JSValue`
  - 语义：断言 ctx 一致；再将其转回 Local 并 handle 一次，确保在返回边界处可达；返回 raw。

> 约束：只有当值与当前 context 一致时才允许返回；跨 context 会 assert。

## 4. glue（rust_glue.rs.j2）中 has_scope 的生成规则

文件：`deps/ridl-tool/templates/rust_glue.rs.j2`

以 singleton method 为例（interface/class 类似）：

- 若 `method.needs_scope=true`：
  - 生成 `scope/env` 并用于：
    - 参数提取（any-like 需要 scope）；
    - any 返回的 `pin_return`。
- 若 `method.needs_scope=false`：
  - 不生成 `scope/env`，直接走纯类型转换路径。

## 5. 最小示例（用于维护时快速判断策略是否正确）

### 5.1 例1：any 作为参数 + 需要 env（needs_scope=true）

IDL：
```ridl
fn echoAny(v: any) -> any;
```

生成 API（关键点）：
- `env: &mut Env<'ctx>` 存在
- 参数：`v: Local<'ctx, Value>`
- 返回：`ReturnAny`

用户实现（必须匹配）：
```rust
fn echo_any<'ctx>(&mut self, env: &mut Env<'ctx>, v: Local<'ctx, Value>) -> ReturnAny {
    env.return_safe(v)
}
```

### 5.2 例2：any 作为参数但不需要 env（needs_scope=false）

IDL：
```ridl
fn logAny(v: any) -> void;
```

若未来某场景被判定为 `needs_scope=false`（例如纯 raw 透传、或策略调整），则生成 API：
- 无 `env`
- 参数：`v: Local<'_, Value>`

注意：此时用户实现里不能调用 `env.return_safe(v)`（因为没有 env）。如果需要 return-safe 或
其他 ctx 相关能力，应让该方法被判定为 needs_scope=true。

## 6. 维护注意事项（踩坑记录）

1. **不要在 filters 层把 any 直接映射成 `Local<'ctx, _>`**：
   - 因为 filters 不知道当前参数所在的方法是否 `needs_scope`，会导致无 env 的方法也泄露 `'ctx`。
2. **绑定 any 参数到 `'ctx` 必须在模板层按 method.needs_scope 做条件渲染**。
3. **模板条件表达式要保守**：Askama 对复杂表达式支持有限（例如 `a and b` 的解析可能踩坑），
   推荐用嵌套 `{% if %}` 或显式的 if/else 分支，确保 `endif` 成对。

## 7. 相关文件索引

- 生成器：
  - `deps/ridl-tool/src/generator/mod.rs`（TemplateMethod.needs_scope / ReturnAny 映射）
  - `deps/ridl-tool/src/generator/filters.rs`（Type::Any => Local<'_, Value>）
- API 模板：
  - `deps/ridl-tool/templates/rust_api.rs.j2`（方法级 'ctx + needs_scope 条件渲染）
- glue 模板：
  - `deps/ridl-tool/templates/rust_glue.rs.j2`（needs_scope 时生成 scope/env；any return 用 pin_return）
- 运行时：
  - `deps/mquickjs-rs/src/env.rs`（Env::return_safe / Env::pin_return）
