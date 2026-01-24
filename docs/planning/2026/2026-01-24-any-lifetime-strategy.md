<!--
status: 已完成
owner: Mi Code
tags: ridl, any, lifetime, mquickjs-rs, glue
-->

# 2026-01-24：any 参数生命周期策略（方案1最终落地）

## 结论（可执行规则）

在 RIDL v1 生成的 Rust API trait 中，对 `any`（Rust 映射为 `Local<Value>`）采用“两段式”生命周期策略：

1. **any 作为参数（param:any）**
   - 默认生成：
     - `any` 参数类型为 `mquickjs_rs::handles::local::Local<'_, mquickjs_rs::handles::local::Value>`
   - 但若该方法 **needs_scope=true**（即该方法签名包含 `env: &mut Env<'ctx>`），则：
     - `any` 参数类型提升为 `Local<'ctx, Value>`

2. **any 作为返回值（return:any）**
   - 为保持 trait **object-safe** 且避免返回类型携带调用点生命周期：
     - `any` 返回类型不再是 `Local<'_, Value>`
     - 统一映射为 `mquickjs_rs::handles::return_safe::ReturnAny`
   - glue 层统一用：
     - `env.pin_return(result)` 返回 `JSValue`

## 为什么需要这样做

- `Env<'ctx>` 以 `&mut Env<'ctx>` 形式传入时，因 Rust 可变引用的**不变性（invariance）**，
  `env.return_safe(v)` 需要 `v: Local<'ctx, _>`。
- 如果 API trait 把 `any` 参数一律生成 `Local<'ctx, _>`，则在 **不需要 scope/env** 的方法上会出现
  “未声明的 `'ctx` 泄露”，导致 API 无法表达。
- 因此必须做到：
  - **只在 needs_scope=true 的方法上**，才把 `any` 参数绑定到 `'ctx`。

## 实现落点（代码位置）

- `deps/ridl-tool/src/generator/filters.rs`
  - `Type::Any` 的基础映射保持为 `Local<'_, Value>`（参数侧不默认引入 `'ctx`）。
- `deps/ridl-tool/templates/rust_api.rs.j2`
  - 在 singleton/interface/class 的方法参数生成处：
    - 若 `method.needs_scope` 且 `p.ty == Type::Any`，则将参数渲染为 `Local<'ctx, Value>`。
    - 否则使用 `p.rust_ty`（即 `Local<'_, Value>`）。
- `deps/ridl-tool/src/generator/mod.rs`
  - `any` 返回值（以及 `Option<any>`）映射为 `ReturnAny`（以及 `Option<ReturnAny>`）。
- `deps/ridl-tool/templates/rust_glue.rs.j2`
  - `return any` 的 glue 路径：`env.pin_return(result)`。

## 对使用方的约束（tests/ 与模块实现）

- 对于 `needs_scope=true` 且参数是 `any` 的方法：
  - 实现签名必须写成 `v: Local<'ctx, Value>`，否则会触发：
    - `argument requires that '1 must outlive 'ctx`（由 `&mut Env<'ctx>` 不变性导致）

## 验证状态

- 已通过：
  - `cargo run -p ridl-builder -- prepare`
  - `cargo test`
  - `cargo run -- tests`
