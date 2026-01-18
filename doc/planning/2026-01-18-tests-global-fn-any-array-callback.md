# 计划：扩展 tests/global/fn 用例覆盖 any/array/callback（2026-01-18）

> 状态：草案（待确认）

## 0. 背景

此前 `tests/global/fn` 只覆盖了最小的 `addInt(int,int)->void` 调用链（JS -> RIDL glue -> Rust impl）。

在本阶段我们已打通 RIDL(any) 的新调用链：
- Rust 边界 `any` 映射为 `mquickjs_rs::mquickjs_ffi::JSValue`（避免生命周期不可表达）
- 对 `needs_scope=true` 的方法，glue 会创建 `Env` 并把 `&mut Env` 作为首参传给 impl

同时，mquickjs-rs 新增了：
- `Env<'ctx>`（持有 `Scope + HandleScope`，作为常用入口）
- `Any<'hs,'ctx>` newtype（基于 Handle<Value>）
- `Array` 类型与 no-holes 语义相关 API（len/get/set/push/pop）

需要在 `tests/global/fn` 中补齐更贴近真实调用的覆盖：
- any 的透传/创建/修改
- array 的 no-holes 行为
- callback（function 交互）的基本调用

## 1. 目标

在不引入硬编码/白名单的前提下，扩展 `tests/global/fn`：
1) 覆盖 RIDL(any) 参数与返回值在 glue/impl 之间的传递正确性。
2) 覆盖 Rust 侧使用 `Env` 创建/读取 JS 值的常见路径。
3) 覆盖 array(no-holes) 的关键语义：
   - `index == len` 允许扩容
   - `index > len` 由引擎抛 TypeError（策略 A：不在 Rust 侧提前拦截）
4) 覆盖 callback 的最小链路：JS 传入 callback，Rust 侧触发调用并返回结果（或把错误透传成 JS 异常）。

## 2. 设计范围（RIDL API 设计）

在 `tests/global/fn/test_fn/src/test_fn.ridl` 的 `singleton TestFn` 中新增下列方法（均为 strict）：

### 2.1 any 相关

- `fn echoAny(v: any) -> any;`
  - 语义：原样透传 `JSValue`。

- `fn makeAnyString(s: string) -> any;`
  - 语义：Rust 侧通过 `env.str(s)` 创建字符串并返回。

- `fn anyToString(v: any) -> string;`
  - 语义：Rust 侧通过 `env.get_string(scope.value(v))` 转 string；失败则抛 TypeError（返回 Err）。

### 2.2 array 相关（覆盖 no-holes）

- `fn makeArrayWithLen(len: int) -> any;`
  - 语义：Rust 侧 `env.array_with_len(len as u32)` 创建 dense array 并返回其 raw JSValue。

- `fn arrayPush(arr: any, v: any) -> int;`
  - 语义：把 `arr` 视为 array，push `v`，返回新长度。
  - 失败：arr 非 array 则 TypeError。

- `fn arraySet(arr: any, index: int, v: any) -> void;`
  - 语义：调用 `Local<Array>::set(&env, index, value)`。
  - 关键：`index > len` 由引擎抛 TypeError（Rust 返回 Err，JS 侧可捕获）。

- `fn arrayLen(arr: any) -> int;`
  - 语义：读取 length。

### 2.3 callback（function 交互）

RIDL 当前对“function 类型”表达方式是 callback：

- `fn callCb1(cb: callback(x: int), x: int) -> int;`
  - 语义：Rust 调用 `cb(x)`，返回 int。

说明：若当前 generator 尚不支持在 Rust 侧实际调用 callback（缺少 callback handle/调用 API），则本条先以“能走通参数提取 + 能安全返回 TypeError”作为第一阶段，后续再补齐真正调用。

## 3. JS 集成用例（tests/global/fn/basic.js）

在现有 `basic.js` 上扩展断言（沿用当前仓库 tests 里常见写法，若无断言库则用简单的 `if (...) throw`）：

1) any 透传：
   - `t.echoAny(123) === 123`
   - `t.echoAny("x") === "x"`

2) makeAnyString/anyToString：
   - `t.anyToString(t.makeAnyString("hi")) === "hi"`

3) array：
   - `let a = t.makeArrayWithLen(2);` 验证 `t.arrayLen(a) === 2`
   - `t.arrayPush(a, 1)` 后长度递增
   - 越界 set：`try { t.arraySet(a, 10, 1); throw new Error("should throw"); } catch(e) { /* ok */ }`

4) callback：
   - `t.callCb1((x)=>x+1, 41) === 42`（若实现阶段可支持）

## 4. Rust 实现（tests/global/fn/test_fn/src/fn_impl.rs）

- impl trait 将根据生成代码变化：
  - 含 any 的方法签名为：`fn xxx(&mut self, env: &mut mquickjs_rs::Env<'_>, ...) -> ...`
  - any/array 参数以 `mquickjs_rs::mquickjs_ffi::JSValue` 传入/返回

实现要点：
- any：直接返回/使用 Env 创建字符串。
- array：
  - `let arr_local = env.scope().value(arr).try_into_array(env.scope())?;`
  - `arr_local.len(env)` / `arr_local.push(env, env.scope().value(v))` 等
  - `arraySet`：调用 `set`，让引擎抛错。

callback：
- 若 generator 已提供 callback 的 Rust 映射（例如某种 JSValue + 调用胶水），按既有模式实现。
- 若尚缺：先在计划确认后进一步调研再落地（不在未确认情况下扩展 generator）。

## 5. 测试与验证

- Rust：`cargo test`（至少覆盖 test_fn crate 编译通过）。
- JS 集成：`cargo run -- tests`。
- 若 callback 调用需要新增底层支持，则必须补对应单元测试。

## 6. 风险与待确认点

1) callback 的“实际调用”是否已被当前 ridl-tool/mquickjs-rs glue 支持：
   - 若不支持，本计划将 callback 相关条目降级为“参数提取 + 返回错误”的最小覆盖，或拆分为后续计划。

---

请你确认：
- 是否接受上述新增 RIDL API 列表（any/array/callback）作为 tests/global/fn 的扩展范围？
- callback 这一条如果暂时无法在 Rust 侧真实调用，你希望：A) 先不加 callback；B) 先加但仅测提取/报错路径；C) 同期补齐 callback 调用能力（这会扩大 scope）。
