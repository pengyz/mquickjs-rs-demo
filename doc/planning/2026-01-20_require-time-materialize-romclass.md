# 2026-01-20：在 require() 时特化递归 materialize ROMClass→ctor

## 背景
当前问题：`new require("m").MyClass()` 失败，`typeof require("...").Foo === "object"`。

原因推断：RIDL 生成的 module namespace / exports 使用 ROM props 挂载 `JS_DEF_CLASS`（ROMClass），但仅对 `globalThis` 表层做过 eager 替换；require() 返回的 module 实例内部未做 ROMClass→ctor 替换，导致导出的 class 仍是 ROMClass（在 JS 侧表现为 object 而非 function ctor）。

在尝试基于引擎属性读取路径做 runtime lazy 替换时，暴露出：
- 访问路径不唯一（可能绕过单个 hook 点），导致覆盖不确定；
- 在属性查找热路径下沉副作用会触发崩溃（重入/暂态不一致风险）。

因此改为新的假设与策略：
> **未替换 ROMClass 的 class 目前唯一来源是 RIDL 引入、且通过 require() 获取的模块对象。**

在该假设下，可以把处理**特化到 require()**：require() 创建 module 实例后，一次性递归/遍历其可见 exports，把 ROMClass 替换为 ctor function，并把结果写回 module 实例（own props 缓存），后续任意访问路径都稳定。

## 目标
- 支持命名空间隔离：`new require("m").MyClass()` 可用。
- 以最小改动获得确定性：不再依赖引擎内部 property lookup 多路径覆盖。
- 不解码 ROM props 布局：使用引擎现有 API/内部 helper 访问属性值。

## 核心设计

### 1) 处理时机
在 `deps/mquickjs-rs/require.c` 的 require() 逻辑里：
- `JS_NewObjectClassUser(ctx, best->module_class_id)` 创建 module 实例后
- 立刻执行 `materialize_module_exports(ctx, obj)`
- 返回 `obj`

这样保证 require() 返回时，导出已是 ctor function。

### 2) 处理范围（遍历边界）
需要覆盖两处：
- **module 实例自身 own props**（如果导出挂在实例上）
- **module 原型上的导出**（当前生成里，module_proto_props 也会挂 `Foo`）

策略：
- 先遍历实例 own props。
- 再遍历其 prototype 的 own props（只处理第一层原型；不递归上溯）。
- 对 prototype 命中的导出，**写回实例 own prop**（缓存语义确定，避免污染 prototype 共享）。

### 3) 递归策略
对导出的属性值 val：
- 若 val 是 ROMClass：用 `stdlib_init_class(ctx, rc)` 得到 ctor，并写回到实例（`JS_DefinePropertyInternal(..., JS_DEF_PROP_HAS_VALUE)` 或等价公开 API）。
- 若 val 是普通 object（namespace 子对象）：递归处理其 own props（以及其 prototype 的第一层），但需要：
  - **循环引用保护**：维护 visited（可用指针集合或临时标记）
  - **深度限制**：默认 max_depth（例如 8 或 16）
  - **白名单/触发条件**：只对从 module 根可达的对象做处理，不扫描全局。

注意：递归只发生在 require() 时，次数与模块导出规模相关。

### 4) API/实现选择（避免 ROM 解码）
实现上优先选择：
- 用 `JS_GetOwnPropertyNames`/等价内部 helper 枚举属性 key
- 用 `JS_GetPropertyInternal`/等价读取属性值
- 替换使用 `JS_DefinePropertyValue`/内部 define helper 写回实例

不直接遍历 ROM props 内存布局。

### 5) 与 ensure_class_ids 的关系
RIDL 生成已有 `ensure_class_ids`，用于保证 class 定义被强制引用/链接。
require() materialize 不依赖它作为运行时机制，只依赖：
- ROMClass 存在 ctor_idx
- stdlib_init_class 可在需要时初始化对应 class

### 6) 语义约束
- 写回策略：统一写回到 require() 返回的 module 实例 own props。
- 原型链语义：prototype 上的导出仍保持只读（不改动），实例上会新增 own prop 覆盖。

## 风险与对策
- **递归导致性能开销**：限制深度；只在 require() 时执行；仅处理 exports 子图。
- **循环引用**：visited 集合或最大节点数上限。
- **异常传播**：任何 JS API 失败返回 exception，require() 直接返回 exception。
- **一致性**：只在 require() 做一次性 materialize，避免多路径访问 hook。

## 验证矩阵
- `cargo run -p ridl-builder -- prepare`
- `cargo test`
- `cargo run -- tests`（重点：`tests/global/require/test_require/tests/basic.js`）

## 待确认点
1. 遍历范围：是否只处理 module 实例 + 其 prototype 一层（默认是）。
2. 递归深度与循环保护参数：max_depth / max_nodes。
3. 写回策略：始终写回实例 own prop（默认是）。
