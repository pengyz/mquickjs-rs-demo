# 规划：在 mquickjs 引擎中引入通用 `gc_mark` 机制（联合图遍历能力）

日期：2026-01-25

## 背景与问题

mquickjs 当前对 user class（`JS_CLASS_USER + N`）仅提供了 C finalizer 表（`c_finalizer_table`）：

- 在 sweep/free 阶段会调用 `ctx->c_finalizer_table[class_id - JS_CLASS_USER](ctx, opaque)`。
- 但在 GC mark 阶段，并不会回调到 embedder 来枚举 `opaque` 所持有的 JS 引用。

同时，`mquickjs.h` 公共 API 允许外部跨调用持有 JS 引用（通过 `JSGCRef` 与 `JS_AddGCRef/JS_PushGCRef` 等接口）。

这导致“跨 JS + native”的对象图无法被当作同一个可达性图来分析：

- 若外部为避免悬垂而“pin/root”某些 JSValue，复杂对象图中很容易出现 **跨边界强引用环**，导致 wrapper 永远可达、finalizer 永不触发，造成泄漏。
- 若外部不 pin/root，则存在 **漏标记导致悬垂** 的风险（native/外部长期保存的 JSValue 被 GC 回收）。

## 目标（完备性定义）

从引擎通用角度引入 `gc_mark` 机制，使 GC 能够对**所有“公共 API 明确允许外部长期持有”的引擎对象/句柄**进行联合图遍历。

本规划将“完备”定义为：

- 任何通过 `mquickjs.h` 公共 API 跨调用持有的引用边，都能在 GC mark 阶段被正确枚举并纳入标记；
- 因此 JS ↔ native ↔ JS 的环在“无外部根”时可整体回收；
- 外部显式 root 的对象可稳定存活（不悬垂）。

非目标（本阶段不做）：

- 并发/增量 GC 的跨语言 barrier 设计；
- 全量 unified heap（native 对象本身进入引擎 GC）。

## 设计原则

1. **QuickJS 风格命名**：使用 `gc_mark` / `mark_value` 等术语，与 finalizer 对称。
2. **引擎通用性**：API 不包含 RIDL、Rust 等使用侧术语，仅以 user class + opaque + context hook 为抽象。
3. **能力最小化**：`gc_mark` 回调中禁止调用普通 JS API，仅允许通过引擎提供的 `mark_value()` 报告边，避免 GC reentrancy。
4. **静态注册兼容**：保持 ROM build（mquickjs-build）静态生成表的模式，新增 `c_mark_table` 与 `c_finalizer_table` 同构。
5. **覆盖外部 roots**：除 user class 外，还必须覆盖 context 级别的外部 roots（例如 JSGCRef 链、context user_data 等潜在持有者）。

## 公共 API 梳理：允许外部长期持有的对象/句柄（以 mquickjs.h 为准）

> 本节作为“完备性基准”。若未来公共头新增/公开新的可持有句柄类型（例如 Atom），需要同步扩展 `gc_mark` 能力接口。

当前 `mquickjs.h` 明确暴露并允许外部长期持有的主要类别：

1) **JSValue**
- 通过 `JSGCRef` + `JS_AddGCRef/JS_DeleteGCRef`（链表 root）
- 或通过 `JSGCRef` + `JS_PushGCRef/JS_PopGCRef`（栈 root）

2) **user class 的 opaque**
- 通过 `JS_SetOpaque/JS_GetOpaque` 绑定到 `JS_NewObjectClassUser` 创建的对象

3) **context 级别 user_data**（潜在持有者）
- `JS_SetContextUserData(ctx, user_data, fin)`
- 当前仅有 finalizer，没有 mark hook；若 user_data 可能持有 JSValue，则必须纳入联合遍历

未见公开：`JSAtom`（当前头文件无 JSAtom typedef / dup/free API），因此本方案不把 Atom 作为“外部可持有对象”的完备性要求。

## 对外 C ABI（建议）

### (A) user class：每类一个 `gc_mark` 回调表（与 finalizer 对称）

在 `mquickjs.h` 中新增：

```c
typedef struct JSMarkFunc {
    void (*mark_value)(const struct JSMarkFunc *mf, JSValue v);
    void *opaque; /* engine private */
} JSMarkFunc;

/* Called during GC mark phase. Must not call JS APIs other than mark_value(). */
typedef void (*JSCMark)(JSContext *ctx, void *opaque, const JSMarkFunc *mf);
```

并扩展 `JSSTDLibraryDef`：

```c
typedef struct {
    const JSWord *stdlib_table;
    const JSCFunctionDef *c_function_table;
    const JSCFinalizer *c_finalizer_table;
    const JSCMark *c_mark_table; /* NEW: per user class gc_mark */
    uint32_t stdlib_table_len;
    uint32_t stdlib_table_align;
    uint32_t sorted_atoms_offset;
    uint32_t global_object_offset;
    uint32_t class_count;
} JSSTDLibraryDef;
```

语义约束：

- `JSCMark` 在 GC 标记阶段调用。
- 回调中 **不得** 进行 JS 调用、分配、抛异常等；仅允许调用 `mf->mark_value(mf, v)`。

### (B) context 级别：embedder roots 的 `gc_mark` hook（覆盖外部持有根）

为覆盖“外部直接持有 JSValue（不挂在 user class 上）”以及 “context user_data 可能持有 JSValue” 的情况，新增 context 级别 hook：

```c
typedef void (*JSContextGCMark)(JSContext *ctx, void *opaque, const JSMarkFunc *mf);
void JS_SetContextGCMark(JSContext *ctx, void *opaque, JSContextGCMark mark);
```

语义约束同上：只能 `mark_value()`。

该 hook 用于枚举：

- embedder 自己维护的跨调用 roots（若存在）；
- context user_data 内部持有的 JSValue（若有）；
- 其它通过公共 API 暴露的外部根集合。

> 备注：`JSGCRef` 链表/栈在引擎内部已有维护位置；它应由引擎在 mark roots 时自动扫描，无需依赖 `JS_SetContextGCMark`。但若 embedder 另有自建 roots 表，则需该 hook。

## 引擎实现（mquickjs.c）

1) **user class 对象扫描**：在 GC mark 扫描对象（`JS_MTAG_OBJECT` 且 `class_id >= JS_CLASS_USER`）时：

- 若 `ctx->c_mark_table[class_id - JS_CLASS_USER]` 非空，则调用。
- 引擎在栈上构造 `JSMarkFunc mf`，其 `mark_value()` 内部转发到引擎私有 `gc_mark(GCMarkState*, JSValue)`。

2) **context 级 hook 调用点**：在 `gc_mark_all()` 或等价的 mark roots 阶段：

- 在扫描完引擎自身 roots（含 JSGCRef 链/栈等）之后，若 `ctx` 上设置了 `JSContextGCMark`，则调用它。

## ROM build 支持（mquickjs-build）

### 需要扩展的数据模型

在 `mquickjs_build.h` 的 `JSClassDef` 中新增字段：

- `const char *gc_mark_name; /* \"NULL\" if none */`

并扩展 `JS_CLASS_DEF(...)` 宏以携带 `gc_mark_name`。

### 生成 user class 两张表

在 `mquickjs_build.c` 中新增 `dump_cmarks()`：

- 生成 `static const JSCMark js_c_mark_table[JS_CLASS_COUNT - JS_CLASS_USER] = { ... }`
- 并在生成的 `const JSSTDLibraryDef js_stdlib = { ... }` 中填入 `js_c_mark_table`

说明：context-level `JS_SetContextGCMark` 属于运行时 API，不由 ROM build 静态表生成。

## 分阶段推进（PoC -> 完整方案）

### 阶段 0：代码阅读与确认（完成条件）

- 确认现状仅有 `c_finalizer_table`，GC mark 未对 user class 提供 hook。
- 确认 `JSGCRef` 机制属于公共 API，且 GC roots 扫描必须覆盖它。

### 阶段 1：PoC（验证 user class `gc_mark` 的联合遍历闭环）

目的：验证“mark 阶段回调 user class 的 gc_mark，可以让跨边界环被正确回收”。

做法：

- 在 `deps/mquickjs/example.c` 新增一个 user class：
  - wrapper A：JS 对象（class_id 为 user class）
  - opaque 指向 native 结构体 `Native`，其中保存一个 `JSValue held;`
- `gc_mark` 回调：调用 `mark_value()` 标记 `held`。
- 构造引用环：
  - A -> opaque -> held(B)
  - B 持有 A（例如 B.some = A）
- 断开所有外部引用后触发 `JS_GC(ctx)`。

验收标准：

- finalizer 能被触发（证明环整体不可达时可回收）。

### 阶段 2：PoC（验证外部 roots：JSGCRef 的可达性正确）

目的：验证“外部跨调用 root 的 JSValue，在 GC 后仍可用”，并作为完备性回归。

做法：

- 在 example 中创建一个对象/字符串 `v`。
- 使用 `JS_AddGCRef(ctx, &ref)` 将其加入 root 链。
- 调用 `JS_GC(ctx)` 后验证 `v` 仍可访问。
- 删除 root：`JS_DeleteGCRef(ctx, &ref)` 后再次 `JS_GC(ctx)`，验证可被回收（以可观测方式）。

验收标准：

- root 生效时不悬垂；
- root 移除后可回收。

### 阶段 3：ROM build 完整接线（引擎通用能力落地）

目的：将 user class `c_mark_table` 变为“静态注册可用”的正式能力。

工作内容：

- 扩展 `mquickjs.h` / `JSSTDLibraryDef`，引入 `c_mark_table`。
- 扩展 `mquickjs_build.h` / `JSClassDef` 增加 `gc_mark_name`。
- 修改 `mquickjs_build.c` 生成 `js_c_mark_table` 并填入 `js_stdlib`。
- 修改 `mquickjs.c` 在 mark 阶段调用 `c_mark_table`。

验收标准：

- `mqjs_ridl_stdlib.h`（或 base build 生成物）中出现 `js_c_mark_table`。
- `js_stdlib` 初始化包含 `js_c_mark_table`。
- 阶段 1 的环回收 PoC 在不做临时接线下通过。

### 阶段 4：context-level hook 落地（覆盖 embedder 自建 roots / user_data）

目的：让 embedder 能在 context 级别报告额外 roots（例如 user_data 内持有的 JSValue）。

工作内容：

- 在 `mquickjs.h` 增加 `JS_SetContextGCMark` API。
- 在 `mquickjs.c` 的 mark roots 阶段调用该 hook。

验收标准：

- example 构造一个 user_data，内部持有一个 JSValue；通过 `JS_SetContextGCMark` 报告它；GC 后仍可用。

## 测试计划（覆盖矩阵 + 可观测性 + 最小回归集）

目标：为“联合图遍历”提供引擎级可重复回归测试，覆盖所有 **公共 API 允许外部长期持有** 的根/边组合。

### 1) 覆盖矩阵

#### A. roots 来源（必须覆盖）

1. **JS roots**：普通 JS 变量/全局对象保持可达
2. **引擎 roots（JSGCRef）**：`JS_AddGCRef/JS_DeleteGCRef` 与 `JS_PushGCRef/JS_PopGCRef`
3. **context roots（embedder hook）**：`JS_SetContextGCMark` 报告的 roots
4. **user class roots**：`JS_CLASS_USER+N` 对象可达时触发 `c_mark_table` 的枚举

#### B. 边类型（必须覆盖）

1. **native -> JSValue（单边）**：opaque 内保存 1 个 JSValue
2. **native -> JSValue（多边）**：opaque 内保存 N 个 JSValue（数组/向量）
3. **环（JS wrapper ↔ held）**：wrapper 通过 opaque 持有 B，B 反向持有 wrapper

#### C. 负例/回归（必须覆盖）

1. **未实现/禁用 gc_mark 时**：opaque 持有的 JSValue 在 GC 后不应被保证存活（用可观测指标证明）
2. **启用 gc_mark 后**：上述 JSValue 必须稳定存活（不悬垂）
3. **移除 root/断开引用后**：对象应可回收（避免引入新的泄漏路径）

### 2) 可观测性（如何断言“回收/存活”）

联合 GC 的测试必须能可靠判断对象是否被回收。建议采用“双信号”但以 finalizer 计数为主：

- **finalizer 计数器（主）**：
  - 为 PoC user class 提供 C finalizer，finalizer 内递增一个全局计数器；
  - 通过 `JS_GC(ctx)` 触发并断言计数变化。

- **值可用性检查（辅）**：
  - 对于应存活的 JSValue，GC 后尝试读取属性/转字符串等；
  - 若被错误回收通常会触发异常或返回非法值（作为辅助观测）。

### 3) 落地形式（优先方案）

优先在 `deps/mquickjs/example.c` 增加非交互 selftest 模式，以便 CI/本地可重复执行：

- `example --selftest-gc-mark`
- 返回码：0 通过 / 非 0 失败

原因：
- 这是引擎级能力验证，不依赖 RIDL/Rust glue；
- 可直接调用 C API 组合 roots/环；
- 可在引擎改动期间快速迭代。

后续可选：增加 Rust harness（`tests/`）调用同样的入口，统一到仓库测试命令。

### 4) 最小回归集（必须实现）

#### 用例 1：user class `gc_mark` 能正确回收跨边界环

- 构造：wrapper A（user class）opaque 持有 `held = B`；并令 `B.some = A`
- 断开所有外部引用（让 A 只通过环可达）
- 调用 `JS_GC(ctx)`

断言：
- finalizer 计数器递增（环整体被回收）

#### 用例 2：JSGCRef root 保活与释放

- 创建 JSValue `v`
- `JS_AddGCRef(ctx, &ref)` root `v`
- `JS_GC(ctx)` 后 `v` 仍可访问（值可用性检查）
- `JS_DeleteGCRef(ctx, &ref)`
- `JS_GC(ctx)` 后 `v` 应允许回收（可通过把 `v` 挂到一个带 finalizer 的 user class 字段来观测回收，或通过计数器间接确认）

断言：
- root 生效时不悬垂
- root 移除后可回收（避免泄漏）

#### 用例 3：context-level `JS_SetContextGCMark` 报告 roots（覆盖 user_data/外部表）

- 构造一个 user_data 结构体，内部保存 JSValue `v`
- 设置 `JS_SetContextGCMark(ctx, user_data, mark_cb)`，在 `mark_cb` 中 `mark_value(v)`
- `JS_GC(ctx)` 后 `v` 仍可访问
- 取消 hook/清空 user_data 后 `JS_GC(ctx)`，对象应可回收（同上用 finalizer 计数辅助）

断言：
- context roots 报告有效
- 移除后可回收

## 风险与缓解

1) 漏标记导致悬垂
- 缓解：在使用侧强制“受控句柄/字段”来存放 JSValue，确保可枚举；提供 debug instrumentation（统计/断言）。

2) 回调中误用 JS API（GC reentrancy）
- 缓解：在文档与头文件注释中明确禁止；debug build 可加入运行期 guard（后续再设计）。

3) ROM 生成链路变更影响面
- 缓解：与 `c_finalizer_table` 同构实现；先在 base build + ridl build 各跑一轮生成与现有测试。

## 相关文件（初步）

引擎：
- `deps/mquickjs/mquickjs.h`（扩展 JSSTDLibraryDef、声明 JSCMark/JSMarkFunc、JS_SetContextGCMark）
- `deps/mquickjs/mquickjs.c`（GC mark 调用点 + mark roots hook 调用点）
- `deps/mquickjs/mquickjs_build.h`（JSClassDef 增加 gc_mark_name、宏扩展）
- `deps/mquickjs/mquickjs_build.c`（生成 js_c_mark_table 并填入 js_stdlib）

PoC：
- `deps/mquickjs/example.c`

---

状态：v2 草案（待讨论确认后再进入实现阶段）。
