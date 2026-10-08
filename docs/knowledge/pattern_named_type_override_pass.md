---
name: pattern-named-type-override-pass
description: RIDL 生成器处理命名类型（struct/enum）的范式：渲染前 override pass，而非改无状态 askama filter
type: pattern
created: 2026-10-08
sources: [deps/ridl-tool/src/generator/mod.rs, docs/planning/2026/2026-10-08-todo-items-resolution.md]
---

**范式**：askama filter（如 `rust_type_from_idl`）是无状态函数，携带不了
聚合上下文（本模块有哪些命名类型）。要按聚合内容改写生成的 Rust 类型，
用**渲染前 override pass**：

1. 收集阶段：从模块定义构建命名类型表（`NamedTypeInfo`，含 struct 字段
   形状 `NamedTypeShape`，供转换生成器递归）。
2. 覆写阶段：遍历模板节点，把方法签名/字段中的裸 `Custom(X)` 改写为
   `crate::api::X` 并把形状挂到节点上（`apply_named_type_rust_ty_overrides`）。
3. **顺序敏感**：`apply_union_rust_ty_overrides` 先行，named-type pass 随后
   （generator_named_types_override_test.rs 钉死）。`(A|B)` 形括号 union 编码
   的 Custom 不得被 named-type 劫持。

**How to apply**：新增"签名里出现聚合级类型"类特性时走同构 pass；
不要给 filter 加参数、不要在 filter 里做全局查找。

**配套纪律**：任何"转换失败静默回退"（如 struct 字段 rust_ty 回退 JSValue、
union 收集静默丢非基元成员）都必须改成硬错误——两个静默面都曾制造
语义错误而不报错。相关：[[gotcha-ridl-silent-generation-degradation]]
