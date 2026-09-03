---
name: ridl-test-coverage-analysis
description: RIDL 工具链测试覆盖分析——当前状态、缺口、优先级
type: reference
created: 2026-09-04
sources: [deps/ridl-tool/tests/, deps/ridl-tool/src/parser/mod.rs]
---

## RIDL 工具链测试覆盖分析（2026-09-04）

### 总体统计

| 组件 | 源码行数 | 测试数 | 覆盖率评估 |
|---|---|---|---|
| 语法（grammar.pest） | 121 行 / 60 规则 | 59 inline + 38 external | ~70% |
| 解析器（parser/mod.rs） | ~800 行 | 59 inline | 中 |
| 代码生成器（generator/） | ~2500 行 | 38 external | 中 |
| 模板（templates/*.j2） | ~600 行 | 5 render tests | 低 |

### 语法规则覆盖矩阵

#### ✅ 已覆盖（有独立测试）

| 规则 | 测试数 | 测试类型 |
|---|---|---|
| module_decl | 8 | 正向+反向+版本格式 |
| interface_def | 2 | 正向+反向 |
| class_def | 3 | 正向+反向+属性 |
| enum_def | 2 | 正向+反向 |
| struct_def | 2 | 正向+msgpack |
| global_function | 2 | 正向+反向 |
| singleton_def | 2 | 正向+反向 |
| import_stmt | 2 | 正向+反向 |
| using_def | 2 | 正向+反向 |
| callback_def | 2 | 正向+反向 |
| opaque_block | 15 | 正向+反向+边界 |
| traced_type | 1 | 正向 |
| union_type | 3 | 正向+复杂 |
| nullable_type | 2 | 正向+复杂 |
| array_type | 1 | 正向 |
| map_type | 1 | 正向 |
| group_type | 1 | 正向 |
| literals | 4 | 每种字面量 |
| identifier | 2 | 正向+关键字冲突 |

#### ❌ 未覆盖（需要补充）

| 规则 | 风险 | 优先级 | 建议测试 |
|---|---|---|---|
| mode_decl | 低 | P2 | 正向+反向（仅综合测试覆盖） |
| proto_var_member | 中 | P1 | 正向+类型验证 |
| proto_readonly_prop | 中 | P1 | 正向+readonly 语义 |
| proto_readwrite_prop | 中 | P1 | 正向+读写语义 |
| class_constructor | 高 | P0 | 正向+参数解析 |
| class_constructor_compat | 高 | P0 | 兼容性格式 |
| normal_prop | 低 | P2 | 正向 |
| null_literal | 低 | P2 | 正向+在默认值中使用 |
| variadic_param | 中 | P1 | 正向+类型验证（generator 有测试） |

### 代码生成器覆盖

#### ✅ 已覆盖

| 功能 | 测试文件 | 测试数 |
|---|---|---|
| opaque struct 生成 | opaque_struct_generation_test.rs | 2 |
| gc_mark 生成 | gc_mark_generation_test.rs | 4 |
| C 头文件渲染 | gcmark_render_test.rs | 1 |
| 类型映射 | filters.rs (inline) | ~20 |
| union 类型 | union_*_test.rs | 4 |
| 错误处理 | error_test.rs | 2 |
| 模式传播 | mode_propagation_*_test.rs | 2 |
| 模块 ID 分配 | module_class_id_allocation_test.rs | 1 |

#### ❌ 未覆盖

| 功能 | 风险 | 优先级 |
|---|---|---|
| Traced<T> 嵌套类型映射 | 高 | P0 |
| C 头文件 JS_CLASS_DEF 参数顺序 | 高 | P0 |
| 端到端编译验证 | 高 | P0 |
| 错误消息质量 | 中 | P1 |
| 模板渲染边界情况 | 中 | P1 |

### 优先补充建议

**P0（立即补充）**：
1. class_constructor / class_constructor_compat 测试
2. Traced<T> 嵌套类型映射测试（Array<Traced<T>>, Map<K, Traced<T>>）
3. C 头文件 JS_CLASS_DEF 9 参数顺序验证
4. 端到端编译测试（ridl → codegen → cargo check）

**P1（近期补充）**：
1. proto_var_member / proto_readonly_prop / proto_readwrite_prop 测试
2. variadic_param 解析测试
3. 错误消息包含位置信息验证
4. 模板渲染边界情况（空 opaque、单字段、多字段混合）

**P2（后续补充）**：
1. mode_decl 独立测试
2. normal_prop 测试
3. null_literal 测试
