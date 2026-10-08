---
name: gotcha-ridl-silent-generation-degradation
description: RIDL 生成器两处静默退化（union 丢成员、struct 字段回退 JSValue）曾制造不报错的语义错误
type: gotcha
created: 2026-10-08
sources: [deps/ridl-tool/src/generator/union_types.rs, deps/ridl-tool/src/generator/mod.rs]
---

**两个已修复的静默退化**（2026-10-08 TODO 处置 Phase D 发现并改为硬错误）：

1. **union 成员静默丢弃**：`union_name_from_members` / `map_union_member`
   对非基元成员（如 `string | Address` 的 `Address`）静默丢弃后按裁剪集
   命名——`fn f(x: string | Address)` 生成只含 String 变体的
   `UnionString`，不报错；传对象运行时 TypeError，传 string 静默通过。
   现已改为 union 成员含 Custom/ClassRef → 生成期报错。

2. **struct 字段 rust_ty 回退 JSValue**：`rust_type_from_idl` 失败时
   字段类型静默变 `JSValue`——嵌套 struct 字段在查表机制接入前会生成
   错误类型而不报错。现已改为硬错误（指明 struct 与字段名）。

**Why**: 生成器的"尽力而为回退"把类型错误从编译期推迟到运行期甚至
无限期潜伏（无使用即无暴露）；类型系统代码里回退 = 语义错误。

**How to apply**: 生成器任何 match 臂想"兜底"时，问一句：这个兜底
产物被使用时是错的吗？是 → 硬错误 + 指明位置。同族地雷仍有一处待办：
class js_fields validator 允许 I64/F32/F64 但 glue 模板 `unreachable!()`
panic（validator/生成器支持集不一致，见 gotcha_quickjs_rom_ridl_mechanism）。
