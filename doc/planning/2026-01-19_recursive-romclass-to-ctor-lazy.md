# 2026-01-19：递归处理 module 导出 class（ROMClass→ctor）的 lazy 方案（规划）

> 目标：让 `new require("m").MyClass()` 在不污染 globalThis 的前提下可用；保持引擎改动最小化；不引入 build 侧“隐藏根”；不做 ROMClass→ctor 的全量深度遍历（避免 ROM props 布局误判）。

## 背景与问题陈述

### 现象（已验证）
- globalThis 最外层的 `class` 导出可正常 `new`。
- module/内层导出的 `class`（如 `m1.Foo`）在 JS 侧读出来是 object（ROMClass），导致 `typeof m1.Foo !== "function"`，`new m1.Foo()` 失败。
- 目前 `tests/global/require/test_require/tests/basic.js` 明确要求：
  - `typeof m1.Foo === "function"`
  - `new m1.Foo()` 可用

### 根因（代码证据）
- `deps/mquickjs/mquickjs.c`：
  - `stdlib_init()` 只遍历 global object 的 (name,val) 表层数组，遇到 `val` 是 ROMClass 则调用 `stdlib_init_class()` 并把返回的 ctor function define 到 globalThis。
  - `stdlib_init_class()` 对 `class_def->props` / `class_def->proto_props` 仅做 `p->props = ...`（挂 ROM props），不会把 ROM props 内部的 `JS_DEF_CLASS` 条目替换为 ctor function。
- `deps/mquickjs/mquickjs_build.c`：
  - ROM props 的 key/value 编码并非简单 int 数组（key 为 atom，value 为 `JS_ROM_VALUE(ident)`，第三字段还编码 `hash_next` + prop_type）。
  - 手写遍历/解码 ROM props 极易误判导致 crash（此前已发生 Unknown error 全线）。

## 设计目标

1) **递归闭环**：module/嵌套导出的 class 只要被访问到，就能 materialize 成 ctor function，最终 `new obj.MyClass()` 工作。
2) **最小改动**：不改 build，不引入隐藏根；运行时改动集中在 property 读取路径。
3) **一次性替换 + 缓存**：只在第一次访问时处理 ROMClass→ctor，并写回到 RAM props（COW），后续访问零额外开销。
4) **覆盖面可证**：确保普通对象属性读取路径均可拦截到，不依赖 opcode 特例。

## 最终方案（固定）

### 核心策略：在 `JS_GetPropertyInternal()` 中对 ROMClass 做 lazy 初始化与写回

在 `deps/mquickjs/mquickjs.c` 的 `JS_GetPropertyInternal()`：

- 找到 own property：
  ```c
  pr = find_own_property(ctx, p, prop);
  if (pr) {
      if (likely(pr->prop_type == JS_PROP_NORMAL)) {
          return pr->value;
      }
      ...
  }
  ```

- 将 `return pr->value;` 替换为：
  1. 取 `val = pr->value`。
  2. 若 `val` 指向 ROMClass 且 `ctor_idx >= 0`：
     - `ctor = stdlib_init_class(ctx, rc)`（它会创建 proto + ctor，并写入 `ctx->class_proto[]/class_obj[]`）。
     - **写回 receiver**：对最初的 receiver `obj`（而不是当前 `p`，因为 `p` 可能是原型对象）执行：
       - `js_update_props(ctx, obj)`（触发 COW 到 RAM）
       - `JS_DefinePropertyInternal(ctx, obj, prop, ctor, JS_NULL, JS_DEF_PROP_HAS_VALUE)`
     - 返回 `ctor`。
  3. 否则返回原 `val`。

> 说明：该方案不需要解析 ROM props layout。属性查找本来就能正确定位 key/value/prop_type，我们只在成功读取到 value 后做一次替换。

### 递归性
- 递归通过“访问链”自然触发：
  - 首次访问 `m.A` 替换 A；之后访问 `m.A.B` 时 B 同样按需替换。
- `stdlib_init_class()` 具备幂等：若 `ctx->class_obj[class_id]` 已存在则直接返回。

### 作用域与语义
- 不修改 globalThis 注入策略：仍只注入 global_object_offset。
- module 命名空间内 class 通过 `new require("m").MyClass()` 使用；不需要额外导出到 globalThis。

## 覆盖面与无例外路径的验证计划

1) 代码审计（grep）：确认所有 JS property 读取 API（`JS_GetProperty*`）最终都调用 `JS_GetPropertyInternal()`。
2) 验证 bytecode 相关路径：确认常见 opcode 的 property read 走 `JS_GetPropertyInternal()` 或其调用点。
3) 本次仅承诺覆盖“普通对象属性读取”（module exports 属于此类），数组/typedarray 等 fast path不在需求范围。

## 风险与防护

- **递归/重入风险**：写回时调用 `JS_DefinePropertyInternal` 可能触发内部逻辑。
  - 防护：仅对 `JS_PROP_NORMAL` 处理，跳过 GETSET；并在写回前检查 receiver 当前属性是否已为 function（避免重复）。
- **原型链语义**：若属性从 proto 读取到 ROMClass，写回 receiver 会让其变成 own property。
  - 这是预期的缓存行为（等价于初始化阶段把导出落到实例/namespace 上）。
  - 若后续发现与语义不符，再讨论“写回 proto 还是 receiver”的策略。

## 测试矩阵（验收标准）

必跑：
- `cargo run -p ridl-builder -- prepare`
- `cargo test`
- `cargo run -- tests`

关键用例：
- `tests/global/require/test_require/tests/basic.js`
  - 期待从 `typeof Foo === "object"` 修复为 `"function"`
  - `new m1.Foo()` 可用

回归：
- 其它 global smoke（diagnostics/fn/types/singleton 等）不应出现 Unknown error。

## 实施步骤（待用户确认后执行）

1) 在 `JS_GetPropertyInternal()` 的 `JS_PROP_NORMAL` 返回点增加 ROMClass→ctor 的 lazy 替换 + 写回。
2) 增加必要的 helper（如 `static inline BOOL is_rom_class_ctor(JSContext*, JSValue, const JSROMClass **out)`），仅用于判定。
3) 跑完整测试矩阵；若出现原型链或重复 define 的边缘问题，再按最小改动迭代。

---

状态：待确认。
