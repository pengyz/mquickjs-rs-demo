# V1-B3：map<K,V>（JS Object 承载）计划

> 日期：2026-01-24
> 状态：草案

## 0. 背景与目标

RIDL 语法已支持 `map<K, V>`，但目前仅能完成解析/归一化，尚未打通 glue 生成与端到端运行。

本计划目标：在 V1 范畴内实现 `map<K,V>` 的 typed boundary：
- Rust API 侧以强类型 `HashMap<K, V>` 表达；
- JS 侧以 **Object** 承载；
- glue 层对 key/value 做严格校验，不允许静默降级。

## 1. 设计决策

### 1.1 JS 承载形态

- JS 侧：普通对象（Object），仅使用 **own enumerable string keys** 表达条目。
- 不支持 Symbol key。
- 不遍历原型链。

> 备注：即使用户声明 `map<i32, V>`，JS 侧 key 仍然以字符串形式出现（对象属性名）。typed 约束由 glue 在解码时实现。

### 1.2 key 类型支持（RIDL K）

K 允许的 RIDL 类型集合（primitive only）：
- `string` / `bool` / `i32` / `i64` / `f32` / `f64`

约束：
- `map<K,V>` 中 K 不允许 `any/object/class/union/optional/array/map/custom`。
- value（V）允许任意已支持类型。

### 1.3 key 解析规则（JS → Rust）

JS → Rust 解码时，key 取自对象 own keys（字符串），并按 K 解析：

- K = string：
  - 直接使用 key 字符串。

- K = bool：
  - 只接受：`"true"` 或 `"false"`（大小写敏感）。

- K = i32：
  - 只接受十进制整数形式（允许前导 `-`）。
  - 解析后必须落在 i32 范围。

- K = i64：
  - 只接受十进制整数形式（允许前导 `-`）。
  - **不做 safe-int 限制**（因为 key 来源是字符串而非 JS number）。

- K = f64 / f32：
  - 只接受有限数（finite）。
  - 禁止 NaN / Infinity / -Infinity。
  - `-0` 归一为 `0`（即解析结果等价于 +0）。
  - f32：解析为 f64 后 cast 为 f32（并要求 finite）。

解析失败：抛 TypeError。

### 1.4 value 解码/编码规则

- value 解码：复用现有类型转换路径（与普通 param 一致）。
- 任一 entry 的 value 解码失败：抛 TypeError。

### 1.5 Rust → JS 编码规则

- 创建新 JS 对象。
- 遍历 Rust `HashMap<K,V>`：
  - key：`K.to_string()` 生成属性名；
  - value：按 V 的 encoder 生成 JSValue；
  - 通过 `JS_SetPropertyStr` 写入。

> HashMap 无序：不承诺 JS 侧 key 顺序。

## 2. 代码生成策略（高层）

### 2.1 Rust API 类型映射

- `map<K,V>` → `std::collections::HashMap<K, V>`

### 2.2 glue：JS → Rust（参数）

- 输入：JSValue v
- 校验：v 必须是 object 且不为 null
- 获取 own keys：使用 `Object.keys(v)` 等价路径（需要确认 mquickjs C API 能否枚举 own keys；若没有直接 API，则用 `JS_GetPropertyStr(ctx, obj, "keys")` 获取全局 Object.keys 并调用）
- 对每个 key：
  - parse key → K
  - get value via `JS_GetPropertyStr(ctx, obj, key)`
  - decode value → V
  - insert into HashMap
- 任一步失败：`js_throw_type_error` 返回 JS exception value

### 2.3 glue：Rust → JS（返回值）

- 创建对象：`JS_NewObject(ctx)`
- 对每个 (k,v)：
  - key_str = k.to_string()
  - encode v
  - `JS_SetPropertyStr(ctx, obj, key_str, js_val)`

## 3. 需要补的 validator 规则

- `Type::Map(key, value)`：
  - key 必须为上述 primitive 集合
  - value 递归 validate
  - 违规则报 SemanticError：提示“map key 仅支持 primitive（string/bool/i32/i64/f32/f64）”。

## 4. 测试矩阵

### 4.1 ridl-tool（生成器/validator 单测）

- validator：
  - reject：`map<object, i32>` / `map<any, i32>` / `map<(string|i32), i32>` / `map<string?, i32>` / `map<array<string>, i32>`
  - accept：`map<i32, string>` / `map<bool, any?>` / `map<f64, (string|i32)?>`

- generator：
  - 生成 glue 中不包含 `?`
  - key parse 失败路径必须走 `js_throw_type_error`

### 4.2 JS 集成（tests/ 下端到端）

新增 global/types 或单独 map 相关模块：
- map<i32, string> param：
  - ok：{ "1": "a", "-2": "b" }
  - err：{ "1.2": "a" } / { "abc": "a" }

- map<bool, i32> param：
  - ok：{ "true": 1, "false": 2 }
  - err：{ "True": 1 } / { "1": 1 }

- map<f64, i32> param：
  - ok：{ "1.5": 1, "-0": 2 }（-0 归一）
  - err：{ "NaN": 1 } / { "Infinity": 1 }

- map<i64, i32> param：
  - ok：{ "9007199254740993": 1 }

- return：Rust 返回 HashMap，JS 侧读取属性值验证

## 5. 实施步骤（建议）

1) validator：加入 map key 限制 + 单测
2) generator：补齐 map<k,v> 的类型映射（Rust API）
3) glue：实现 map param/return 的编解码
4) 端到端测试：JS case + Rust impl
5) 全量回归：prepare + cargo test + cargo run -- tests
