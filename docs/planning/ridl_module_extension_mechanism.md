# RIDL Module Extension Mechanism（设计方案）

> 本文档用于澄清：RIDL 扩展机制与 mquickjs ROM/标准库导入机制之间的关系；给出一个**完备**且**可裁剪**（ridl feature 关闭即消失）的 module 扩展初始化方案。
>
> 约束：本文档只以仓库当前代码（`deps/mquickjs/mquickjs.c`、`deps/mquickjs/mquickjs_build.c`、聚合生成物）为依据。

---

## 1. 背景与问题陈述

当前 JS smoke 用例出现 exports 不可见（例如 `ping/Foo/mping/MFoo` 为 `undefined`）。

我们最初尝试通过在 `JS_RIDL_StdlibInit` 阶段对 `module_proto` 执行 exports materialize（instance 或 proto 版本）来修复，但发现仍然失败。

本方案的目标不是“修一个点”，而是**把 ROM→runtime 的导入闭环与 RIDL 扩展接入点讲清楚并补全**，避免反复踩坑。

---

## 2. 严格 rootcause（代码级证明）

### 2.1 exports 的静态定义确实存在于 module 的 ROMClass.proto_props

RIDL 聚合生成的 `mquickjs_ridl_register.h` 为 module class 生成：

- `JS_CLASS_DEF(..., proto_props = js_*_module_proto_props, ...)`
- 其中 `js_*_module_proto_props` 包含 `JS_CFUNC_DEF("ping", ...)`、`JS_PROP_CLASS_DEF("Foo", ...)` 等 exports。

因此，exports 的定义源在构建期已固化到 ROMClass 的 `proto_props`。

### 2.2 运行时 stdlib_init 不会递归导入 `__ridl_modules` 内部条目

`deps/mquickjs/mquickjs.c` 的 `stdlib_init` 实现如下（关键部分）：

```c
static void stdlib_init(JSContext *ctx, const JSValueArray *arr)
{
    JSValue name, val;
    int i;

    for(i = 0; i < arr->size; i += 2) {
        name = arr->arr[i];
        val = arr->arr[i + 1];
        if (JS_IsObject(ctx, val)) {
            val = stdlib_init_class(ctx, JS_VALUE_TO_PTR(val));
        } else if (val == JS_NULL) {
            val = ctx->global_obj;
        }
        JS_DefinePropertyInternal(ctx, ctx->global_obj, name,
                                  val, JS_NULL,
                                  JS_DEF_PROP_HAS_VALUE);
    }
}
```

可严格推导：

- `stdlib_init` 仅遍历 global object 的顶层 `(name,val)` ROM 数组。
- 若 `val` 是 object，仅调用一次 `stdlib_init_class`。
- **不存在对 `val`（例如 `global.__ridl_modules` 这个对象）的 `props` 做进一步展开/递归遍历的逻辑**。

而 `__ridl_modules` 在聚合生成物中是一个 object namespace（`JS_OBJECT_DEF("RidlModules", ...)`），其内部 props 才包含 module entries（每个 entry 的 value 是 module 的 ROMClass）。

因此：module ROMClass 在 context 初始化的 stdlib 导入阶段**不会被触达**，也就不会执行 `stdlib_init_class` 来创建/补全 module 的 `class_obj/class_proto` 与 `proto_props`。

### 2.3 结论

> exports 定义在 ROMClass.proto_props 中存在，但由于 stdlib_init 不递归导入 `__ridl_modules` 内部结构，module ROMClass 未 materialize，导致 module prototype 未被挂载 exports，从而测试失败。

---

## 3. ROM 与扩展能力的关系（统一心智模型）

### 3.1 构建期静态体系的两条主通道

1) ROM/object graph（`stdlib_table`）：描述对象图、属性表（props/proto_props），可共享只读。
2) `c_function_table`：描述 C 函数入口（通过 idx 引用）。

新增 C 能力（新函数入口、新 class）必须在构建期进入静态体系；运行时只能“装配/暴露”既有能力。

### 3.2 ROM 的 COW 行为与本问题的关系

ROM props 可能在被写入时转为 RAM（COW）。但本问题不是 RAM 化导致读取失效，而是：**module ROMClass 从未导入，exports 根本未安装到运行时 proto**。

---

## 4. 方案目标与设计原则

### 4.1 目标

- 仅针对 **module class**（require_table 中的 module）补全导入。
- 在用户 JS 执行前完成一次性 materialize。
- ridl feature 关闭时，整套机制随之裁剪，不影响 mquickjs 独立编译。
- 不允许重入：同一 `JSContext` 上初始化入口只允许调用一次，第二次必须失败（显式错误）。

### 4.2 设计原则

- **对外不暴露 ROMClass 结构布局**：Rust/外部不解析 ROMClass。
- materialize 入口以 `JSValue` 作为 ROMClass 句柄参数（`JSValue` 是 public 类型）。
- 避免使用 `class_id -> 反查 ROMClass` 的隐式推导；由生成物直接提供 ROMClass 句柄。

---

## 5. 方案：RIDL 侧一次性 module class materialize（ridl 可裁剪）

### 5.1 核心接口（引擎侧）

“扶正”并稳定化：

- `JSValue JS_MaterializeROMClass(JSContext *ctx, JSValue val);`

约束：
- `val` 必须是一个 ROMClass 句柄（ROM 指针编码在 `JSValue` 中）。
- 该函数内部走旧机制（等价 `stdlib_init_class`）完成 class_obj/class_proto 的创建与 `proto_props` 挂载。

> 注意：此接口虽位于 `mquickjs.h`，但参数为 `JSValue`，不暴露内部结构；因此不破坏模块封装性。

### 5.2 聚合生成物需要提供的静态表

仅针对 module classes，生成一个表：

- `module_class_id -> romclass_val(JSValue)`

其中 `romclass_val` 是该 module 对应的 ROMClass 句柄（构建期可确定）。

### 5.3 RIDL 侧初始化入口（一次性、不可重入）

在 ridl 生成的 C 代码中提供一个初始化函数（名称可不暴露到引擎层，位于 ridl 侧）：

- `int ridl_module_extensions_init_once(JSContext *ctx);`

行为：
- 内部维护 ctx-local 的 once 标记。
- 第一次调用：遍历静态表，对每个 module `romclass_val` 调用 `JS_MaterializeROMClass(ctx, romclass_val)`。
- 第二次调用：直接返回错误（不可重入）。

### 5.4 Rust 侧调用时机

仅在 ridl feature 打开时：
- 在 `Context` 初始化流程（用户 JS 执行前）调用 `ridl_module_extensions_init_once(ctx)`。

ridl feature 关闭时：
- 不生成/不链接上述静态表与入口函数，整套机制随 ridl 一起裁剪。

---

## 6. 正确性与完备性说明

### 6.1 为什么该方案完备

- rootcause 是 module ROMClass 未 materialize。
- 本方案显式枚举所有 module ROMClass（require_table 范围），并在用户 JS 前逐一 materialize。
- materialize 使用 ROMClass 的权威定义（proto_props），保证 exports 进入 canonical proto。

### 6.2 为什么不需要 Rust 解析 ROMClass

- ROMClass 句柄以 `JSValue` 形式传递，解析与导入都在引擎内部完成。
- Rust 只负责在正确时机调用一次入口函数。

### 6.3 为什么不依赖 stdlib_init 递归

- stdlib_init 明确不递归，无法覆盖 `__ridl_modules` 内部 module entries。
- 本方案绕开该限制，直接对 module ROMClass 做一次性 materialize。

---

## 7. 开放问题（需实现前确认）

### 7.1 `romclass_val(JSValue)` 的生成来源（实现选型）

本方案要求为每个 module class 提供 `romclass_val: JSValue`（其内部指向 `JSROMClass` 的 ROM 指针），以便调用 `JS_MaterializeROMClass(ctx, romclass_val)` 走旧机制导入。

这里存在两个实现路径：

#### 选项 A：由 ROM builder 输出 `JSValue` 常量（推荐，已选定）

由 **mquickjs-build/ROM builder** 在生成 `stdlib_table` 的同一轮中（同一套 ident 分配）额外输出一个 C 源文件，导出每个 module 的 ROMClass 句柄常量：

- `const JSValue js_ext_romclass__test_require_1_0; /* JS_ROM_VALUE(<offset>) */`

要求：
- 仅覆盖 module classes（`js_ridl_require_table` 中的 module entries）。
- 符号命名使用 **module_full_name**（含版本，如 `test.require@1.0`），并进行规范化：将非 `[A-Za-z0-9_]` 字符替换为 `_`。
- 常量值由 ROM builder 直接写入 `JS_ROM_VALUE(<offset>)`，其中 `<offset>` 必须来自 ROM builder 当次构建生成的 ident（权威来源）。

优点：
- ROMClass 句柄与 stdlib ROM table 同源生成，offset 不漂移。
- 运行时无需解析 `__ridl_modules` 对象，不依赖 stdlib_init 的递归行为。
- materialize 输入就是目标 ROMClass，链路最短，确定性最高。

代价/前置条件：
- mquickjs-build 需要新增一个输出产物（C 源文件）到稳定 include_dir（与 `mquickjs.h` 同目录）。
- `mquickjs-rs` 的 build.rs 需要使用 `cc` 在编译期将该 C 文件编译并链接进 crate。
- 为支持 ridl-off（或未来某些 profile 不生成该文件）的可裁剪性：`mquickjs-rs` 采用“文件存在则编译，不存在则跳过”的策略（不强制 base/include 也生成）。

#### 选项 B：运行时从 `__ridl_modules` 取 class value（JSValue）作为 romclass_val

生成阶段不导出 ROM offset，而是在 `ridl_module_extensions_init_once(ctx)` 内通过 public API 取到：

- `globalThis.__ridl_modules["test.require@1.0"]`

该 property value 在 ROM table 中是由 `JS_PROP_CLASS_DEF(name, &module_class_def)` 编码而来，在 build 阶段会变成 `JS_DEF_CLASS` → `define_class` → ROMClass，因此运行时读到的 value（若仍为 ROM 指针）即可作为 `romclass_val`。

优点：
- 生成器不需要接触 ROM offset（不需要额外与 mquickjs_build 的 ident 对齐）。

风险/不确定性：
- 取值路径依赖 `__ridl_modules` 对象是否已在 global 上可见、以及 property value 在导入后是否仍保持为 ROM 指针形式。
- 若导入阶段把该 value 替换为 ctor/function 或其它 runtime 结构，则需要额外适配。

> 说明：从当前聚合产物可知，`js_ridl_require_table` 仅携带 `module_class_id` 与 ensure_class_ids，不携带 ROMClass 句柄；因此若走选项 B，需要显式从 `__ridl_modules` 读取。

#### 选型结论（建议）

- 优先选项 A（直接导出 romclass JSValue 常量），以获得最短链路与最高确定性。
- 选项 B 可作为 fallback（当生成器难以稳定获取 ROM offset 时）。

### 7.2 once 标记的存放位置

建议放在 ridl context 扩展结构中，保证 per-ctx 语义。

### 7.3 错误返回约定

不可重入时返回何种错误码（-1 或特定码）。

---

## 8. 本方案的范围与非目标

- 范围：仅解决 module class 的导入完备性，确保 exports 可见性。
- 非目标：不在此方案内讨论 exports materialize 的内部实现细节（instance/proto），也不讨论 class_id 分配方案。
