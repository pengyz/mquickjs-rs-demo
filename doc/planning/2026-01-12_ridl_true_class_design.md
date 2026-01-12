# 目标

实现 RIDL 的 `class`（真 class），严格对齐 **mquickjs** 的静态 stdlib/build 机制（而非 QuickJS 运行时注册模型）。

核心能力：
- `new` 构造实例（JS 侧构造函数 + prototype methods）。
- 通过 `opaque` 绑定 Rust 实例指针，支持实例方法调用与属性访问。
- 通过 C finalizer 在 GC/ctx free 时释放实例（只做内存释放，不调用 JS API）。
- 支持 `proto property`：**per-JSContext** 共享态（不同实例共享一份数据）；访问性仅用 `readonly` 控制是否可写。

约束：
- C API 注册必须编译期完成；不得引入运行时注册/白名单/硬编码 singleton 名称。
- 生成物需要符号保活（keep-alive），避免静态链接裁剪。

# 事实依据（mquickjs 源码）

来自 `deps/mquickjs/mquickjs_build.h` / `deps/mquickjs/mquickjs.h` / `deps/mquickjs/mquickjs.c`：
- class 描述：`JSClassDef` + `JS_CLASS_DEF(...)` + `JS_PROP_CLASS_DEF(...)`；由 build 工具生成 ROM 表。
- user class 实例：`JS_NewObjectClassUser(ctx, class_id)`。
- this/opaque：`JS_GetClassID(ctx, val)` / `JS_SetOpaque(ctx, val, opaque)` / `JS_GetOpaque(ctx, val)`。
- finalizer：`typedef void (*JSCFinalizer)(JSContext *ctx, void *opaque)`；且注释明确 finalizer 禁止调用 JS API。
- ctx 创建时由 ROM 定义初始化 class 相关对象（prototype/constructor），并持有 `c_finalizer_table`。
- JSValue 的生命周期管理 **不是引用计数**，而是通过 GC root 机制：
  - 栈式：`JS_PushGCRef/JS_PopGCRef`（宏 `JS_PUSH_VALUE/JS_POP_VALUE`）
  - 列表式：`JS_AddGCRef/JS_DeleteGCRef`

结论：
- class id 与 class/proto 对象创建 **均由 build-time 静态生成/ctx 初始化流程决定**，不需要我们运行期额外注册。
- 我们能控制的是 `JSClassDef` 里的 class_props/proto_props/parent/finalizer_name 等描述。
- any/object 的“可保存性”必须通过 GCRef（pin/unpin）来提供。

# RIDL 语义设计

## 1) class fn
- `class` 内所有 `fn` 都视为 **prototype methods**（定义点在 proto_props）。
- 运行期仍可操作实例：通过 `this_val -> opaque` 获取 Rust 实例。
- 不引入 `proto fn` / `static fn`（避免语言化，且无必要）。

## 2) property
- 默认 `property`：实例态（每实例一份数据）。
  - JS 侧仍通过 prototype 上的 getter/setter 访问。
  - getter/setter 通过 `this_val -> opaque` 调用 Rust 实例 trait。

- `proto property`：共享态（per-JSContext 多实例共享）。
  - JS 侧通过 prototype 上的 getter/setter 访问。
  - getter/setter 通过 `ctx user_data -> ContextInner -> ridl_ext` 获取 per-ctx shared state，再调用 Rust trait。

- `readonly`：只控制是否生成 setter（set = NULL），不区分 Rust/JS。

## 3) any/object（JSValue）映射
- 采用二层模型：借用型 + 可保存型（pinned）。

### 借用型：`ValueRef<'ctx>`
- 用于 glue 调用栈内的临时视图（argv/this）。
- 不保证跨调用边界安全，禁止长期保存。

### 可保存型：`PinnedValue<'ctx>`
- 内部通过 `JS_AddGCRef` 将 JSValue 固定为 GC root（pin）。
- Drop 时 `JS_DeleteGCRef`（unpin）。
- 允许用户在同一 Context 生命周期内跨调用保存。

# RIDL 语法/AST 变更（最小侵入）

- 在 Property 修饰符中新增 `proto`。
- 语法形式：
  - `proto readonly property token: string;`
  - `proto property token: string;`
- 约束：
  - `proto` 仅允许出现在 `class` 的 property 上。
  - 初版禁止 `proto` 与 `const` 同时出现（后续再讨论语义）。

# 代码生成（端到端）

## A. C 侧聚合注册（`mquickjs_ridl_register.h`）

为每个 class 生成：
1. `proto_props`：
   - methods：`JS_CFUNC_DEF` / `JS_CFUNC_MAGIC_DEF`（如需要 magic）
   - properties：`JS_CGETSET_DEF` / `JS_CGETSET_MAGIC_DEF`
   - 末尾 `JS_PROP_END`

2. `JSClassDef`：
   - `static const JSClassDef js_<class>_class = JS_CLASS_DEF(
       "<ClassName>", <ctor_arity>, js_<class>_constructor,
       <CLASS_ID>, /* class_props */ NULL, /* proto_props */ js_<class>_proto_props,
       /* parent */ NULL,
       js_<class>_finalizer
     );`

3. 导出到 global：
   - `JS_PROP_CLASS_DEF("<ClassName>", &js_<class>_class)`

### CLASS_ID 生成与对齐
- class id 的整数值由 build-time 生成。
- 选定方案：额外导出一个头文件 `mqjs_ridl_class_id.h`（由 mquickjs-build 写入 include_dir），用 `enum { RIDL_CLASS_xxx = <int>, ... };` 的方式暴露。
- bindgen（在 mquickjs-rs/build.rs）需要 `-I include_dir` + `-include mqjs_ridl_class_id.h` 以便 Rust glue 能使用 `RIDL_CLASS_*` 常量。

## B. Rust 侧（每模块 OUT_DIR）

### 1) `<module>_api.rs`
- 为每个 class 生成实例 trait：
  - `trait <ClassName>Class { ... }`
  - 构造器返回：`Box<dyn <ClassName>Class>`（调用侧不需要碰 JSValue/class_id）。
  - any/object 参数：`ValueRef<'ctx>`；需要保存的 API 返回 `PinnedValue<'ctx>`（按需求逐步扩展）。
- 若该 class 声明了 `proto property`，生成共享态 trait：
  - `trait <ClassName>Proto { ... }`

> 注意：不生成任何 `todo!()` 的 impl 骨架。

### 2) `<module>_glue.rs`
生成入口：
- `js_<class>_constructor(ctx, this_val, argc, argv) -> JSValue`
  - 解析参数
  - 调用 `crate::impls::<ClassName>::constructor(...) -> Box<dyn <ClassName>Class>`
  - 包装为 `<ClassName>Opaque` 并转 raw 指针
  - `obj = JS_NewObjectClassUser(ctx, CLASS_ID)`
  - `JS_SetOpaque(ctx, obj, opaque_ptr)`
  - return obj

- `js_<class>_<method>(ctx, this_val, argc, argv) -> JSValue`
  - `JS_GetClassID(ctx, this_val)` 校验
  - `opaque = JS_GetOpaque(ctx, this_val)`
  - 调 Rust trait 方法

- `js_<class>_get_<prop>` / `js_<class>_set_<prop>`
  - 实例态：走 this->opaque
  - proto 态：走 ctx-ext（per-ctx shared state）

- `js_<class>_finalizer(ctx, opaque)`
  - 仅做 drop（禁止 JS API）

### 3) Rust Opaque 承载（trait object + drop）
每个 class 生成 wrapper：
- `struct <ClassName>Opaque { v: Box<dyn <ClassName>Class> }`
- ctor：`Box::into_raw(Box::new(<ClassName>Opaque { v }))`
- method glue：`&mut (*p).v`
- finalizer：`drop(Box::from_raw(p))`

共享态类似：
- `struct <ClassName>ProtoOpaque { v: Box<dyn <ClassName>Proto> }`
- 由 ctx-ext 持有并在 ctx drop 时释放。

## C. symbols keep-alive
- aggregated_symbols.rs / symbols.rs 必须引用：
  - ctor/method/getset/finalizer 的 extern 符号
- 避免静态链接裁剪生成入口。

# 运行时与生命周期

- 实例生命周期：由 JS GC / ctx free 触发 class finalizer -> drop opaque。
- finalizer 禁止调用 JS API；因此只允许释放内存/资源。
- proto property 共享态生命周期：per-JSContext，跟随 ctx user_data/ctx-ext drop。
- any/object 的可保存性：必须通过 `PinnedValue`（GCRef pin）实现，且严格绑定同一 Context。

# 需要新增/修改的最小 API 面

## 1) deps/mquickjs-sys（绑定 mquickjs.h）
至少需要暴露：
- `JS_NewObjectClassUser`
- `JS_SetOpaque`
- `JS_GetOpaque`
- `JS_GetClassID`
- `JSGCRef` 与 `JS_AddGCRef/JS_DeleteGCRef`（用于 `PinnedValue`）
- `mqjs_ridl_class_id.h` 中的 `RIDL_CLASS_*` enum 常量（通过 bindgen include）。

## 2) deps/mquickjs-rs
- 在 `mquickjs_rs::mquickjs_ffi` 层 re-export 以上符号与 class id 常量。
- 提供 `ValueRef<'ctx>` / `PinnedValue<'ctx>` 的安全封装（基于 GCRef）。

## 3) deps/ridl-tool
- parser/ast：PropertyModifier 增加 Proto。
- grammar：支持 `proto` 修饰符。
- generator：
  - module generation：处理 IDLItem::Class（生成 api/glue）。
  - aggregate C header：输出 class def + global prop。
  - symbols：保活 class 入口。

# 验收

- Rust：`cargo test -q` 通过。
- JS runner：`cargo run -q -- tests` 通过。
- 新增一个最小 class demo RIDL：
  - 可 `new` 多个实例
  - `proto readonly property token: string` 在多个实例间共享（同一 ctx）
  - 实例 drop 能触发 finalizer（通过计数或日志验证；finalizer 不能调用 JS API）
  - any/object：参数可读；需要保存时使用 `PinnedValue`（GCRef pin）

# 状态
- [ ] 进行中
