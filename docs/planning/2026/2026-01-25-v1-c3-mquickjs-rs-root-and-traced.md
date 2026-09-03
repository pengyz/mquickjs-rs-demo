# 规划：在 mquickjs-rs / RIDL 层落地“统一 tracing GC”（Root<T> + Traced 字段）

日期：2026-01-25

## 背景

引擎层已引入两类通用 mark 能力：

- user class `gc_mark`：在 mark 阶段回调 embedder，枚举 opaque 内持有的 JSValue 边。
- context-level `JS_SetContextGCMark`：在 mark roots 阶段回调 embedder，枚举 context 级 roots。

在 mquickjs-rs 当前实现中：

- `Local<'ctx, T>` 只是 JSValue 视图，不是 root。
- `Global<T>` 通过 `JSGCRef` 链入引擎 roots（`JS_AddGCRef/JS_DeleteGCRef`），因此“跨调用保存必须 Global”。

新需求：

- 用户不需要理解/手动使用 mark。
- 用户在 Rust 结构中保存 JSValue 时：
  - 对象内字段（挂在某个 JS user class 上）可以不使用 Global；
  - 异步任务/全局队列等“堆外结构”也可能持有 JSValue，需要稳定保活且可撤销。

并需要通过“刻意制造 A↔Rust↔B 强引用环”验证：

- 在没有额外根时可回收（不会因过度 root 泄漏）。
- 在异步任务持有根时可保活；释放根后可回收。

## 目标

1) 在 mquickjs-rs 引入 `Root<T>`：

- 作为跨 await / 跨对象持有 JSValue 的主路径句柄；
- 用户不感知 mark；
- Drop 自动撤销根；
- 不要求使用 `Global<T>`（JSGCRef）作为唯一方案。

2) 在 RIDL 生成侧引入 `Traced<T>` 字段语义：

- 用于 user class 的 opaque 内保存 JSValue 字段；
- 由生成代码自动实现 `gc_mark` 枚举这些字段；
- 用户不手写 mark。

3) E2E 测试覆盖：

- 场景：A 是 RIDL 生成 user class；opaque 持有 B；B 回指 A；异步任务（Root）也可能临时持有 B。
- 断言：
  - 释放 Root 后环可回收（finalizer 触发）；
  - 若 Root 不释放则 finalizer 不触发（对照，证明泄漏来自根管理而非 GC 能力不足）。

## 设计概览

### 1) Root<T>（mquickjs-rs）

- Root 的实现采用“roots registry + context gc_mark”路径。
- Context 创建时安装：`JS_SetContextGCMark(ctx, opaque, mark_fn)`
  - `opaque` 指向 `RootsRegistry`（挂在 `ContextInner`）。
  - `mark_fn` 遍历 registry 中当前存在的 roots，逐个 `mark_value(mf, value)`。

#### 1.1 RootsRegistry 数据结构（建议）

- 使用 `Mutex<Slab<JSValue>>` 或 `Mutex<Vec<Option<JSValue>>>`。
- Root<T> 保存一个 key/slot id：
  - `Root::new(scope, local)`：插入并返回 Root。
  - `Drop`：移除 slot（置空）。
- 额外能力（debug）：
  - 统计当前 root 数量；
  - 可选 dump（不纳入稳定 API）。

#### 1.2 Root<T> 的语义

- `Root<T>` 允许跨 async/跨调用存活，但必须满足：
  - 不跨 Context（通过 ctx_id 校验）；
  - 不跨线程（默认 !Send/!Sync，除非明确保证 Context 线程模型）。

### 2) Traced<T>（RIDL 生成 + mquickjs-rs 内部类型）

- `Traced<T>` 用于“对象内字段”：只能出现在 RIDL 生成的 opaque struct 中。
- 它不 root（不进入 RootsRegistry），也不要求 JSGCRef。
- 它依赖 owning JS 对象（A）的 `gc_mark`：
  - 当 A 可达时，gc_mark 会被调用并标记 `Traced` 字段中的 JSValue（例如 B）。
  - 当 A 不可达时，gc_mark 不会被调用，字段不会阻止回收。

### 3) 与 Global<T> 的关系

- `Global<T>` 保持兼容（既有 API 不破坏）。
- 推荐文档/示例将“跨 await 持有”迁移到 `Root<T>`。
- `Global<T>` 作为底层/特殊场景仍可用（例如需要与现有 JSGCRef 生态对齐）。

## 测试计划（必须自动化）

### 1) 引擎自检（已存在）

- 继续通过 `cargo run -p ridl-builder -- selftest-gc-mark` 覆盖引擎级约束。

### 2) mquickjs-rs 单测

- `Root<T>`：创建 root → 触发 GC → 值仍可用；Drop 后 → 不再保活（通过 finalizer/可达性验证）。

### 3) RIDL E2E（JS 集成）

新增一个测试模块（建议 `tests/global/gc/`）：

- 暴露一个 RIDL class `GcNode`：
  - native opaque 内含 `Traced<Value>` held；
  - 提供 `setHeld(v)`、`clearHeld()`、`finalizerCount()`。
- 测试用例：
  1) 构造 A = new GcNode(); B = {}；B.back = A；A.setHeld(B)
  2) 创建 Root<B> 模拟 async 持有；
  3) 释放 JS 侧引用，只保留 Root；触发 GC → 不回收；
  4) drop Root；触发 GC → 回收（finalizerCount 增长）。

## 交付清单

- mquickjs-rs：新增 `Root<T>` + RootsRegistry + Context 初始化安装 ctx gc_mark。
- RIDL：支持 `Traced<T>` 字段并自动生成 user class gc_mark。
- tests：新增 E2E JS 集成用例 +（可选）mquickjs-rs 单测。
- 文档：更新开发指引，推荐使用 Root/Traced。

## 状态

- 状态：待确认 → 待实现
