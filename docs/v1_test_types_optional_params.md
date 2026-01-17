# Optional(T) 参数支持（V1 合规）—测试矩阵与 RIDL 示例

本文档对应下一轮修复：为 v1 glue 增加 **Optional(T) / nullable 参数** 的解码支持。

> 背景：当前 v1 glue 对 `Option<T>` 参数会直接生成 `compile_error!("v1 glue: unsupported parameter type...")`，导致 `T?` 无法作为参数使用。

## 1. 语义确认（已对齐）

- RIDL 是严格语言：即便在 default 模式，也不做“形式类型转换”。
- strict 是额外更严限制（当前主要是禁用 any）；未来可扩展更多限制点。
- 本轮聚焦 Optional(T) 的参数解码，先覆盖 default。

### 1.1 Optional(T) 参数解码规则（default）

对 `v: T?`（Rust trait 侧为 `Option<T>`）：

- `null` -> `None`
- `undefined` -> `None`
- 其他值 -> 必须能按 T 的规则解码为合法 T
  - 解码成功：`Some(T)`
  - 解码失败：`TypeError`

特别确认（你已明确）：
- `string?` 输入 number => **TypeError**（不允许 toString）


## 2. 测试矩阵（Phase A / test_types 扩展项）

### 2.1 `string?` 参数

RIDL：`fn echoStringNullable(v: string?) -> string?;`

JS 断言：
- `TestTypes.echoStringNullable(null) === null`
- `TestTypes.echoStringNullable(undefined) === null`
- `TestTypes.echoStringNullable('hi') === 'hi'`
- `TestTypes.echoStringNullable(123)` 抛 TypeError

> 备注：字符串包含 `\u0000` 的截断语义已经在 `string` 参数用例里覆盖，这里不必重复。

### 2.2 `int?` 参数

RIDL：`fn echoIntNullable(v: int?) -> int?;`

JS 断言：
- `echoIntNullable(null) === null`
- `echoIntNullable(undefined) === null`
- `echoIntNullable(123) === 123`
- `echoIntNullable(1.5)` 抛 TypeError（default 下 int 不接受非整数 number；不做截断/取整）


## 3. 完整 RIDL 示例（拟）

文件：`tests/global/types/test_types/src/test_types.ridl`

在现有基础上追加：

```ridl
singleton TestTypes {
    fn echoBool(v: bool) -> bool;
    fn echoInt(v: int) -> int;
    fn echoDouble(v: double) -> double;
    fn echoString(v: string) -> string;

    fn echoStringNullable(v: string?) -> string?;
    fn echoIntNullable(v: int?) -> int?;

    fn echoAny(v: any) -> any;

    // union 仍保持 TODO（后续实现）
}
```


## 4. JS 用例规划（拟）

文件：`tests/global/types/test_types/tests/basic.js`

追加断言：
- `echoStringNullable(null/undefined/'hi'/123-throw)`
- `echoIntNullable(null/undefined/123/1.5-待确认)`


## 5. 需要你确认的唯一问题（用于锁定 int? 的行为）

在 default 模式下：
- `int`/`int?` 输入 `1.5`：
  - A) TypeError（严格拒绝非整数 number）
  - B) 允许并按 JS_ToInt32 的行为转换（截断/取整）

你选 A 还是 B？
确认后我再开始实施（改 glue 生成器 + 恢复 RIDL + 补 JS 测试 + 全量验证，并 amend 到 WIP）。
