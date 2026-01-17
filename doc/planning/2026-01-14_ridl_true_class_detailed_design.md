# 设计：RIDL True Class（真 class）+ Proto Property（per-JSContext 共享）（详细设计）

日期：2026-01-14

状态：进行中（用于确认；未确认执行前不进入实现）

> 本文是对 `doc/planning/2026-01-14_ridl_class_plan.md` 的“设计细化版”。
> 目标是把关键路径、关键数据结构与关键接口契约讲清楚，方便你评审后进入实现。

---

## 1. 术语与约束

### 1.1 术语
- **glue**：生成的 QuickJS-facing C ABI 入口（`js_*`），负责 JSValue<->Rust 转换、receiver/opaque/ctx-ext 解析、抛异常。
- **api**：生成的纯 Rust trait/类型（impl 层要实现/调用），不触碰 JS C API。
- **impls**：用户手写实现模块（模块 crate 内），被 glue 调用。
- **ctx-ext / ridl_ext**：每个 `JSContext` 对应的扩展状态，存于 `mquickjs-rs` 的 `ContextInner.ridl_ext_ptr`。
- **thin-pointer**：可以安全通过 `*mut c_void` 往返的薄指针。`*mut dyn Trait` 是 fat pointer，禁止直接存入。

### 1.2 核心约束（不可违反）
- C API 注册不可运行时动态完成：必须由 build/ROM 产物静态决定。
- 禁止白名单/硬编码模块名驱动聚合与注册。
- finalizer 禁止调用 JS API（只允许释放 Rust 内存/资源）。
- 跨 FFI 的指针必须是薄指针（c_void round-trip 必须安全）。

---

## 2. 现状基线（依赖的既有机制）

### 2.1 Context 与 ctx-ext 的现行模型
`mquickjs-rs` 中：
- `ContextInner` 持有：
  - `ridl_ext_ptr: UnsafeCell<*mut c_void>`
  - `ridl_ext_drop: UnsafeCell<Option<unsafe fn(*mut c_void)>>`
- 应用侧（本 repo 的 `src/context.rs`）在创建 `Context` 后调用：
  - `crate::ridl_runtime_support::ridl_context_init(ctx)`（仅 feature `ridl-extensions`）
- `ridl_context_init`（生成物）负责：
  - 安装 `RidlCtxExtVTable`
  - 分配 `CtxExt` 并通过 `ContextInner::set_ridl_ext(ptr, drop_fn)` 存入
  - 初始化 singleton slots

### 2.2 class 相关 FFI 已存在且形态明确
当前 bindgen（目标目录里的生成物）显示：
- `JS_GetClassID(ctx, val) -> c_int`
- `JS_SetOpaque(ctx, val, opaque: *mut c_void)`
- `JS_GetOpaque(ctx, val) -> *mut c_void`
- `JS_NewObjectClassUser(ctx, class_id) -> JSValue`
并且 `deps/mquickjs-rs/src/class.rs` 已封装：
- `Context::new_class_object(class_id)`
- `ClassObject::set_opaque/get_opaque/class_id`

这意味着 v1 不需要引入新的 mquickjs C API；主要工作在生成器与 ctx-ext 聚合侧。

---

## 3. 总体架构（端到端关键路径）

### 3.1 端到端时序（启动/初始化/调用/销毁）

#### 3.1.1 进程准备阶段（构建期）
1. `ridl-builder` 选择 RIDL modules（SoT：registry/manifest）。
2. `ridl-tool` 生成：
   - `mquickjs_ridl_register.h`（包含 class defs + global prop 注入 + singleton defs）
   - `ridl_symbols.rs`（Rust 侧保活引用）
   - 每个模块 crate 的 `*_api.rs`/`*_glue.rs`/`*_symbols.rs` 等（由模块 build.rs include）
3. `mquickjs-build` 消费 `mquickjs_ridl_register.h` 生成 ROM：
   - 生成/安装 class defs、constructor、proto entries
   - 生成 `mqjs_ridl_class_id.h`（class-id 数值权威来源）

#### 3.1.2 运行时初始化阶段（每个 JSContext）
1. `mquickjs_rs::Context::new()` 创建 JSContext 并安装 user_data（Arc<ContextInner>）。
2. 应用调用 `ridl_context_init(ctx)`：
   - 安装 ctx-ext vtable（一次/进程）
   - 分配 `CtxExt`，调用 `ContextInner::set_ridl_ext(ptr, drop_fn)`
   - 初始化 singleton slots（通过 `RidlCtxExtWriter + RidlErasedSingletonVTable`）
   - **新增**：初始化 proto backing（per class，一次/ctx）并存入 `CtxExt`

#### 3.1.3 JS 调用阶段
- `new Foo(a,b)`：
  1) QuickJS 调用 glue 入口 `js_<module>_<foo>_constructor`
  2) glue 转换参数 -> 调用 `crate::impls::foo_constructor(...)`
  3) 得到 `Box<dyn crate::api::FooClass>`，包装为 thin-pointer 存入 JS object opaque
  4) 返回 JS object

- `foo.method(x)`：
  1) QuickJS 调用 glue 入口 `js_<module>_<foo>_<method>`
  2) glue 校验 receiver：`JS_GetClassID(ctx,this) == RIDL_CLASS_*`
  3) glue 取 opaque（thin-pointer）-> `&mut dyn FooClass`
  4) glue 转换参数 -> 调用 trait 方法 -> 转换返回值

- `foo.prop`（实例 property）：
  1) QuickJS 调用 `get_<prop>` / `set_<prop>` glue
  2) receiver 走 this->opaque（同 method）

- `foo.token`（proto property）：
  1) QuickJS 调用 `get_proto_<prop>` / `set_proto_<prop>` glue
  2) receiver 不使用 this->opaque，而是：`ctx -> ContextHandle -> inner.ridl_ext_ptr -> ctx-ext proto lookup`
  3) 调用 `dyn FooProto` 的 getter/setter

#### 3.1.4 销毁阶段
- class instance：GC 或 ctx teardown 触发 finalizer：drop 该对象对应的 thin-pointer allocation。
- ctx teardown：
  - `ContextInner::drop` 调用 `ridl_ext_drop(ptr)`
  - `CtxExt::drop_all()` 释放：
    - 所有 singleton slots（`ErasedSingletonSlot::drop_in_place()`）
    - 所有 proto backing（新增）

---

## 4. 关键数据结构与 ABI 契约

### 4.1 class 实例 opaque：thin-pointer 规范

#### 4.1.1 存储形式
- 目标：把 `Box<dyn FooClass>` 变成可通过 `*mut c_void` 往返的薄指针。
- 采用与 singleton 一致的模式：**存储指向 `Box<dyn Trait>` 的指针**。

具体：
- ctor glue 中：
  - `let inst: Box<dyn FooClass> = crate::impls::foo_constructor(...);`
  - `let holder: Box<Box<dyn FooClass>> = Box::new(inst);`
  - `let p: *mut Box<dyn FooClass> = Box::into_raw(holder);`  // thin pointer
  - `JS_SetOpaque(ctx, obj, p as *mut c_void)`

- method/getset glue 中：
  - `let p = JS_GetOpaque(ctx, this_val) as *mut Box<dyn FooClass>;`
  - `if p.is_null() => throw TypeError("missing opaque")`
  - `let inst: &mut dyn FooClass = &mut **p;`

- finalizer glue 中：
  - `let p = JS_GetOpaque(ctx, val) as *mut Box<dyn FooClass>;`
  - `if !p.is_null() { drop(Box::from_raw(p)); }`

#### 4.1.2 receiver 校验
- glue 必须先做：`JS_GetClassID(ctx, this_val) == RIDL_CLASS_*`
- 否则抛 `TypeError("invalid receiver")`

> 注意：本 fork 的 `JS_GetOpaque` 不携带 class_id 参数，因此 receiver 校验必须显式做。

### 4.2 class-id 常量：自解析稳定导出

#### 4.2.1 SoT
- SoT 是 `mqjs_ridl_class_id.h`（mquickjs-build 生成）。
- `mquickjs-rs/build.rs` 已实现：读该头文件，自解析 enum，写 `OUT_DIR/ridl_class_id.rs`。

#### 4.2.2 稳定路径契约（生成器只依赖这一条）
- `mquickjs_rs::ridl_class_id::RIDL_CLASS_<module>_<class>`

实现方式（设计约束）：
- 在 `mquickjs-rs` crate 根引入：
  - `pub mod ridl_class_id { include!(concat!(env!("OUT_DIR"), "/ridl_class_id.rs")); }`
- 且不要在生成器里引用 bindgen 输出的 `mquickjs_ffi::RIDL_CLASS_*`。

### 4.3 proto property：ctx-ext vtable 扩展（关键设计）

#### 4.3.1 为什么不复用 singleton slot
- singleton slot 是 `ErasedSingletonSlot`（带 drop_fn、指向 singleton 实例）。
- proto backing 的语义是“per-class shared state”，并且不需要对外暴露 `create/drop vtable`。
- 复用 slot 会引入命名空间冲突与语义混淆；也会迫使 proto state 走 vtable create/drop，增加耦合。

#### 4.3.2 vtable 扩展接口（ABI 冻结）

在 `mquickjs_rs::ridl_ext_access::RidlCtxExtVTable` 增加：
```rust
pub get_proto_by_name: unsafe extern "C" fn(
    ext_ptr: *mut c_void,
    name_ptr: *const u8,
    name_len: usize,
) -> *mut c_void,
```

ABI 语义（冻结）：
- `ext_ptr`：必须是当前 `JSContext` 对应的 ctx-ext 指针；调用方保证非空。
- `name_ptr/name_len`：指向 ASCII key（见 4.3.3）；调用方保证该 slice 在调用期间有效。
- 返回值：
  - **非空**：表示找到对应 proto backing，返回一个可 round-trip 的薄指针（`*mut c_void`）。
    - 该指针的真实类型为：`*mut Box<dyn <Class>Proto>`（即“指向 box 的指针”，thin-pointer）。
    - glue 侧负责按当前 class 的 `<Class>Proto` trait 进行 cast，并解引用为 `&mut dyn <Class>Proto`。
  - **空**：未找到（key 不存在/未初始化），调用方（glue）负责抛 `TypeError("missing proto state")`。

错误处理与职责边界（冻结）：
- vtable 实现不得抛 JS 异常；只返回 null/非 null。
- vtable 实现不得分配新的 proto backing；proto backing 的创建在 `ridl_context_init` 完成。
- proto backing 的释放由 ctx teardown 的 `ridl_ext_drop` 统一完成（vtable 不负责 drop）。

#### 4.3.3 name-key 规范（冻结）

目标：为 `get_proto_by_name(ext_ptr, name_ptr, name_len)` 提供一个**跨 crate / 跨模块全局唯一**、**稳定**、**只依赖生成器输入**的 key。

规范：
- key 必须完全由生成器决定（禁止手写/运行时拼接），并且在同一 app 的聚合产物中全局唯一。
- key **必须是 ASCII**（`[0-9A-Za-z_:.]` 的子集），便于在 C/Rust/模板中无歧义使用。
- key 格式：
  - `proto:<module_ns>::<class_name>`

其中：
1) `module_ns`（模块命名空间）来源优先级（冻结）：
   - 若 RIDL 文件存在 `module foo.bar` 声明：
     - `module_ns = "foo.bar"`（原样点分）
   - 否则：
     - `module_ns = <crate_name>`（来自 cargo package name，经 normalization；见下）

2) `<crate_name>` normalization（冻结）：
   - 将所有非 `[A-Za-z0-9_]` 字符替换为 `_`（例如 `-` -> `_`）。

3) `<class_name>`：
   - 使用 RIDL 中声明的 class 名称（区分大小写），但在 key 中同样要求 ASCII。
   - 若出现非 ASCII（理论上应由 RIDL 语法层禁止）：生成器报错并指出来源文件/位置。

全局唯一性保证：
- `module_ns` + `class_name` 的组合在 RIDL registry 级别不允许重复；如重复，生成器必须报错并列出冲突来源。

#### 4.3.4 CtxExt 布局（聚合生成物）
`CtxExt` 中新增字段：
- `proto_<sanitized_key>: *mut c_void` 或 `Option<NonNull<c_void>>`
- drop 时：把该字段解释为 `*mut Box<dyn Proto>` 并 drop。

初始化时（`ridl_context_init`）：
- 对每个 class（存在 proto property 才生成）：
  - 调用模块提供的构造函数（稳定入口）：`crate::impls::<class>_proto_create() -> Box<dyn Proto>`
  - `Box::new(proto) -> *mut Box<dyn Proto>`
  - 存入对应字段

#### 4.3.5 glue 访问 proto backing
- getter/setter glue：
  - `ContextHandle::from_js_ctx(ctx)`
  - `ext_ptr = h.inner.ridl_ext_ptr()`，若 null => TypeError("missing ridl ext")
  - `p = ridl_get_proto_by_name(ext_ptr, key)`，若 null => TypeError("missing proto state")
  - cast：`p as *mut Box<dyn FooProto>` -> `&mut dyn FooProto`

> 这里 glue 需要一个 runtime helper：`ridl_get_proto_by_name`（对称于现有 `ridl_get_erased_singleton_slot_by_name`）。

---

## 5. 生成器（ridl-tool）改造点（关键路径）

> 重要约束：module 级别生成产物必须收束。
> class 的胶水代码必须归入 module 的 `glue.rs`；trait/纯 Rust 声明必须归入 module 的 `api.rs`。
> 因此 v1 不新增任何“class 专用输出文件”。

### 5.1 输出形态与模板组织（收束策略）
- 继续以 module 为单位生成两类 Rust 产物：
  - `rust_api.rs.j2` -> `api.rs`
  - `rust_glue.rs.j2` -> `glue.rs`
- class 支持的渲染方式：
  - 在 `rust_api.rs.j2` 中扩展对 `IDLItem::Class` 的渲染分支，输出 `<Class>Class` / `<Class>Proto` trait。
  - 在 `rust_glue.rs.j2` 中扩展对 `IDLItem::Class` 的渲染分支，输出 ctor/method/get/set/finalizer 的 `js_*` C ABI 入口。

说明：仓库当前确实存在 `rust_class_api.rs.j2` / `rust_class_glue.rs.j2`，且被 `rust_api.rs.j2` / `rust_glue.rs.j2` include。
但它们不应被视为“额外生成物”，而仅是模板内部的组织手段；如后续需要进一步收束，可把其内容内联进两个主模板。

### 5.2 class glue 的对齐点（并入 glue.rs）
- 不再依赖 `crate::generated::api`，统一走 `crate::api`。
- ctor 不再要求 `new_<class>(ctx,this,argv_vec)`：改为纯 Rust 参数，复用 `rust_glue.rs.j2` 的参数转换与报错 helper。
- receiver 校验：`JS_GetClassID(ctx, this_val) == mquickjs_rs::ridl_class_id::RIDL_CLASS_*`。
- opaque 往返：thin-pointer（`*mut Box<dyn Trait>`）规范，finalizer 仅做 drop。

### 5.3 class api 的对齐点（并入 api.rs）
- 生成 `trait <Class>Class { ... }`（实例方法/实例 property get/set）。
- 当且仅当存在 proto property：生成 `trait <Class>Proto { ... }`（同样只含纯 Rust 方法）。

关于 property 到 trait 方法的映射规则（确认版）：
- 实例 property：
  - getter：`fn get_<prop>(&mut self) -> T`
  - setter：`fn set_<prop>(&mut self, v: T) -> ()`
- proto property：
  - 在 `<Class>Proto` 上生成同名 get/set（同样不带 ctx）。

ctx 获取约定（确认版）：
- trait 方法 **不**显式接收 `&mquickjs_rs::Context`。
- 如确有需要（线程相关 ctx 等），impl 内通过 `mquickjs_rs::Context::current()` 获取当前线程的上下文句柄/对象。

### 5.3 C 聚合：proto property 的 C 侧 getter/setter 注入
- `mquickjs_ridl_class_defs.h.j2` 目前只注入 methods。
- 需要扩展：
  - 对每个 property（实例/ proto）生成对应 `JS_CGETSET_DEF` 条目挂到 `proto_funcs[]`。
  - readonly property 只生成 getter，setter 为 NULL。

并生成对应 extern 声明：
- `JSValue js_<module>_<class>_get_<prop>(...)`
- `JSValue js_<module>_<class>_set_<prop>(...)`（如非 readonly）

### 5.4 符号保活
- Rust 侧 `ridl_symbols.rs`（聚合）必须引用：
  - class ctor/method/get/set/finalizer（Rust 导出）
  - `js_<module>_<class>_class(void)`（C keep-alive stub；保证 class_def/proto_funcs 不被裁剪）

---

## 6. 错误模型与诊断（关键失败模式）

### 6.1 统一错误策略
- glue 负责把错误映射到 JS 异常。
- v1 只需要 `TypeError`（现有 helper：`JS_ThrowError(...JS_CLASS_TYPE_ERROR...)`）。

### 6.2 关键失败模式与建议报错信息

1) receiver 错误（method/getset 用在非本类对象上）
- 条件：`JS_GetClassID != RIDL_CLASS_*`
- 报错：`"invalid receiver"`

2) opaque 缺失
- 条件：`JS_GetOpaque == NULL`
- 报错：`"missing opaque"`

3) ctx-ext 未初始化
- 条件：`ContextHandle::from_js_ctx(ctx)` 返回 None 或 `inner.ridl_ext_ptr() == NULL`
- 报错：`"missing ridl ext (call ridl_context_init)"`

4) proto state 缺失
- 条件：`get_proto_by_name` 返回 NULL
- 报错：`"missing proto state"`

5) 参数类型错误
- 条件：转换失败
- 报错：包含参数序号，如：`"arg1: expected string"`

---

## 7. 内存/生命周期/线程安全约束

### 7.1 生命周期
- class instance：由 JS GC/ctx teardown 触发 finalizer drop。
- proto backing：由 ctx-ext 统一持有与 drop（跟随 JSContext 生命周期）。

### 7.2 线程安全
- JSContext 不是线程安全对象。
- 所有 glue 调用都发生在创建该 JSContext 的线程（由宿主保证）。
- ctx-ext 内的数据结构不要求 Send/Sync，但应避免跨线程共享。

### 7.3 安全边界（unsafe 约束）
- 所有 `*mut c_void` 的 cast 必须与初始化时写入的真实类型一致。
- thin-pointer 的 drop 必须 exactly once：
  - class：finalizer drop
  - proto：ctx-ext drop
  - singleton：slot drop
- 不允许把 JSValue 长期保存到 Rust 对象中（除非后续引入 `Global<Value>`/GCRef root 方案并明确绑定 ctx）。

---

## 8. 已确认的接口细节（影响生成器接口）

1) trait 方法参数是否包含 `ctx: &mquickjs_rs::Context`
- **已选：方案 B**：trait 方法不带 `ctx`。
- 如需线程相关 ctx：impl 内通过 `mquickjs_rs::Context::current()` 获取。

2) proto backing 的粒度
- **已选：每个 class 一份**（`FooProto` 包含多个 proto property 的 get/set）。

---

## 9. 状态
- [x] 第 8 节接口细节已确认。
- [ ] 本文待你确认“进入实现”。确认后，按 AGENTS.md 规则再开始编码与测试。
