# GC 功能测试完善设计（TDD 驱动）

日期：2026-09-03

## 目标

完成 mquickjs-rs 的 GC Root 体系，通过 TDD 方法论确保质量：
1. 扩展 RIDL 语法支持 opaque 字段声明（彻底方案）
2. 修复当前测试的弱断言和验证逻辑分离问题
3. 扩展测试覆盖到正向/反向/边界场景
4. TDD 驱动实现 Traced<T>，验证与 Root<T> 的行为差异

## 项目现状

### 已完成
- ✅ `Root<T>` 完整实现（Context-level gc_mark + RootsRegistry）
- ✅ RIDL 测试模块（test_gc_root_cycle）
- ✅ JS 端到端测试（basic.js）
- ✅ Rust Drop finalizer 计数机制

### 问题诊断

**问题 A - 测试覆盖度不足**：
- 当前只有一个 basic.js 测试
- 缺少反向测试（验证不该回收时没回收）
- 缺少边界情况（多环、长链、空 Root）

**问题 B - 测试断言太弱**：
```rust
// tests/gc_root_cycle.rs:79
assert!(FINALIZER_COUNT.load(Ordering::SeqCst) >= 0); // 永远通过！
```

**问题 C - 测试组织不清晰**：
- Rust 测试未真正验证回收
- JS 测试验证了回收，但 Rust 测试没用上

**问题 D - 缺少 TDD 流程**：
- Traced<T> 规划已久，但未实现
- 需要先写测试，再驱动实现

### 未完成（目标）
- ❌ `Traced<T>` 类型定义
- ❌ RIDL 代码生成器支持 Traced<T> 字段
- ❌ 自动生成 per-class gc_mark
- ❌ 完善的测试覆盖

## 设计方案

### 整体架构

**四阶段实施**（最彻底方案）：
0. **阶段 0**（3-5 天）：扩展 RIDL 语法支持 opaque 字段声明
1. **阶段 1**（0.5-1 天）：修复当前测试
2. **阶段 2**（1-1.5 天）：扩展测试覆盖
3. **阶段 3**（2-3 天）：TDD 驱动 Traced<T> 实现

**测试策略**：
- **主战场**：Rust 集成测试（`tests/gc_root_cycle.rs`）
- **保留**：JS 端到端 smoke test（`basic.js`）
- **工具层**：测试辅助模块（GcTestHarness）

---

## 阶段 0：扩展 RIDL 语法（3-5 天）

### 目标

让 RIDL 支持在 `.ridl` 文件中声明 opaque 字段，为自动生成 gc_mark 提供类型信息。

### 0.1 新增 RIDL 语法

**当前 RIDL 语法**：
```ridl
class Node {
    fn setHeld(v: object) -> void;
    fn clearHeld() -> void;
}
```

**扩展后语法**：
```ridl
class TracedNode {
    opaque {
        held: Option<Traced<Value>>
        self_obj: Option<Traced<Value>>
    }
    
    fn setHeld(v: object) -> void;
    fn clearHeld() -> void;
}
```

**语法规则**：
- `opaque { ... }` 块可选，位于 class 内部
- 字段声明：`field_name: Type`
- 支持的类型：
  - `Traced<T>` - 需要 gc_mark 标记
  - `Option<Traced<T>>` - 条件标记
  - 其他 Rust 类型（如 `i32`、`String`）- 不生成 gc_mark
- 一个 class 最多一个 opaque 块

### 0.2 RIDL Parser 改动

**文件**：`deps/ridl-tool/src/parser.rs`

**新增 AST 节点**：
```rust
// AST 定义
pub struct Class {
    pub name: String,
    pub opaque_fields: Vec<OpaqueField>,  // ← 新增
    pub methods: Vec<Method>,
    // ...
}

pub struct OpaqueField {
    pub name: String,
    pub ty: Type,
}

pub enum Type {
    Traced(Box<Type>),      // Traced<T>
    Option(Box<Type>),      // Option<T>
    Named(String),          // Value, i32, etc.
    // ...
}
```

**解析逻辑**：
```rust
fn parse_class(&mut self) -> Result<Class> {
    // ... 解析 "class Name {"
    
    let mut opaque_fields = Vec::new();
    let mut methods = Vec::new();
    
    while !self.check(TokenKind::RBrace) {
        if self.match_keyword("opaque") {
            opaque_fields = self.parse_opaque_block()?;
        } else if self.match_keyword("fn") {
            methods.push(self.parse_method()?);
        } else {
            return Err("expected 'opaque' or 'fn'");
        }
    }
    
    // ...
}

fn parse_opaque_block(&mut self) -> Result<Vec<OpaqueField>> {
    self.expect(TokenKind::LBrace)?;
    let mut fields = Vec::new();
    
    while !self.check(TokenKind::RBrace) {
        let name = self.expect_ident()?;
        self.expect(TokenKind::Colon)?;
        let ty = self.parse_type()?;
        fields.push(OpaqueField { name, ty });
    }
    
    self.expect(TokenKind::RBrace)?;
    Ok(fields)
}

fn parse_type(&mut self) -> Result<Type> {
    let name = self.expect_ident()?;
    
    match name.as_str() {
        "Traced" => {
            self.expect(TokenKind::Lt)?;
            let inner = self.parse_type()?;
            self.expect(TokenKind::Gt)?;
            Ok(Type::Traced(Box::new(inner)))
        }
        "Option" => {
            self.expect(TokenKind::Lt)?;
            let inner = self.parse_type()?;
            self.expect(TokenKind::Gt)?;
            Ok(Type::Option(Box::new(inner)))
        }
        _ => Ok(Type::Named(name)),
    }
}
```

### 0.3 RIDL Codegen 改动

**生成 opaque struct 定义**：

```rust
// deps/ridl-tool/src/codegen.rs
fn generate_opaque_struct(class: &Class) -> TokenStream {
    if class.opaque_fields.is_empty() {
        // 使用默认 empty struct
        return quote! {
            pub struct #opaque_name;
        };
    }
    
    let field_defs: Vec<_> = class.opaque_fields.iter().map(|f| {
        let name = &f.name;
        let ty = type_to_tokens(&f.ty);
        quote! { pub #name: #ty }
    }).collect();
    
    quote! {
        pub struct #opaque_name {
            #(#field_defs),*
        }
    }
}

fn type_to_tokens(ty: &Type) -> TokenStream {
    match ty {
        Type::Traced(inner) => {
            let inner_tokens = type_to_tokens(inner);
            quote! { mquickjs_rs::Traced<#inner_tokens> }
        }
        Type::Option(inner) => {
            let inner_tokens = type_to_tokens(inner);
            quote! { Option<#inner_tokens> }
        }
        Type::Named(name) => {
            let ident = syn::Ident::new(name, Span::call_site());
            quote! { #ident }
        }
    }
}
```

**生成 gc_mark 函数**：

```rust
fn generate_gc_mark(class: &Class) -> Option<TokenStream> {
    let traced_fields: Vec<_> = class.opaque_fields.iter()
        .filter(|f| is_traced_field(&f.ty))
        .collect();
    
    if traced_fields.is_empty() {
        return None;
    }
    
    let mark_statements: Vec<_> = traced_fields.iter().map(|f| {
        let name = &f.name;
        if is_optional(&f.ty) {
            quote! {
                if let Some(ref field) = this.#name {
                    mark_value(mf, field.as_raw());
                }
            }
        } else {
            quote! {
                mark_value(mf, this.#name.as_raw());
            }
        }
    }).collect();
    
    Some(quote! {
        pub unsafe extern "C" fn gc_mark(
            _ctx: *mut mquickjs_sys::JSContext,
            _obj: mquickjs_sys::JSValue,
            opaque: *mut std::os::raw::c_void,
            mf: *const mquickjs_sys::JSMarkFunc,
        ) {
            if opaque.is_null() || mf.is_null() {
                return;
            }
            
            let this = &*(opaque as *const Self);
            if let Some(mark_value) = (*mf).mark_value {
                #(#mark_statements)*
            }
        }
    })
}

fn is_traced_field(ty: &Type) -> bool {
    match ty {
        Type::Traced(_) => true,
        Type::Option(inner) => is_traced_field(inner),
        _ => false,
    }
}

fn is_optional(ty: &Type) -> bool {
    matches!(ty, Type::Option(_))
}
```

### 0.4 模板更新

**C 头文件模板**：
```c
// deps/ridl-tool/templates/mquickjs_ridl_register.h.j2
static JSClassDef js_{{ class.name }}_class = {
    .class_name = "{{ class.name }}",
    .finalizer = js_{{ class.name }}_finalizer,
    {% if class.has_gc_mark %}
    .gc_mark = js_{{ class.name }}_gc_mark,
    {% else %}
    .gc_mark = NULL,
    {% endif %}
};
```

**Rust FFI 注册**：
```rust
// 生成的注册代码
#[no_mangle]
pub unsafe extern "C" fn js_{{ class.name }}_gc_mark(
    ctx: *mut JSContext,
    obj: JSValue,
    opaque: *mut c_void,
    mf: *const JSMarkFunc,
) {
    {{ opaque_struct }}::gc_mark(ctx, obj, opaque, mf);
}
```

### 0.5 测试

**单元测试**：
```rust
// deps/ridl-tool/tests/parser_test.rs
#[test]
fn test_parse_opaque_block() {
    let input = r#"
        class Node {
            opaque {
                held: Option<Traced<Value>>
                count: i32
            }
            fn test() -> void;
        }
    "#;
    
    let class = parse_class(input).unwrap();
    assert_eq!(class.opaque_fields.len(), 2);
    assert_eq!(class.opaque_fields[0].name, "held");
    assert!(matches!(class.opaque_fields[0].ty, Type::Option(_)));
}

#[test]
fn test_generate_gc_mark_with_traced_fields() {
    let class = Class {
        name: "Node".into(),
        opaque_fields: vec![
            OpaqueField {
                name: "held".into(),
                ty: Type::Traced(Box::new(Type::Named("Value".into()))),
            },
        ],
        methods: vec![],
    };
    
    let gc_mark = generate_gc_mark(&class);
    assert!(gc_mark.is_some());
    // 验证生成的代码包含 mark_value 调用
}

#[test]
fn test_no_gc_mark_without_traced_fields() {
    let class = Class {
        name: "Simple".into(),
        opaque_fields: vec![
            OpaqueField {
                name: "count".into(),
                ty: Type::Named("i32".into()),
            },
        ],
        methods: vec![],
    };
    
    let gc_mark = generate_gc_mark(&class);
    assert!(gc_mark.is_none());
}
```

**集成测试**：
```ridl
// deps/ridl-tool/tests/fixtures/gc_mark.ridl
class TestNode {
    opaque {
        traced_field: Traced<Value>
        optional_field: Option<Traced<Value>>
        plain_field: i32
    }
    
    fn dummy() -> void;
}
```

运行 ridl-tool 生成代码，验证：
- ✅ opaque struct 定义正确
- ✅ gc_mark 函数生成
- ✅ 只标记 Traced 字段，忽略 plain_field

---

## 阶段 1：修复当前测试（0.5-1 天）

### 1.1 强化 Rust 测试断言

**目标**：让 `tests/gc_root_cycle.rs` 真正验证回收行为。

**现状问题**：
```rust
// 当前代码（第 79 行）
assert!(FINALIZER_COUNT.load(Ordering::SeqCst) >= 0); // 任何值都通过
```

**解决方案**：

```rust
#[cfg(feature = "ridl-extensions")]
#[test]
fn gc_root_cycle_collectable_after_root_drop() {
    mquickjs_rs::ridl_bootstrap!();
    let mut ctx = mquickjs_rs::Context::new(1024 * 1024).expect("create context");
    
    // 记录初始 finalizer 计数
    let initial_count = get_finalizer_count(&mut ctx);
    
    // 完全在 JS 侧创建对象和环（避免 Rust/JS 边界问题）
    ctx.eval(r#"
        globalThis.testB = {};
        globalThis.testA = TestGc.makeNode();
        testA.setHeld(testB);
        testB.back = testA;
    "#).expect("create cycle in JS");
    
    // Rust 侧获取 B 的引用并创建 Root
    let token = ctx.token();
    let scope = token.enter_scope();
    let b = ctx.eval_jsvalue("testB").expect("get testB");
    let b_local: Local<'_, Value> = scope.value(b);
    let b_root = Root::new(&scope, b_local);
    
    // 此时：b_root 持有 B，A 被 JS 引用
    // 验证：有 Root 时不回收
    gc(&mut ctx);
    let count_with_root = get_finalizer_count(&mut ctx);
    assert_eq!(count_with_root, initial_count,
        "Object should NOT be collected while Root exists");
    
    // 释放 JS 侧的 A 引用，只保留 b_root
    ctx.eval("a = null; testB = null;").unwrap();
    
    // 验证：有 Root 时仍不回收（Root 保活整个环）
    gc(&mut ctx);
    let count_still_rooted = get_finalizer_count(&mut ctx);
    assert_eq!(count_still_rooted, initial_count,
        "Cycle should NOT be collected while Root exists");
    
    // 释放最后的 Root
    drop(b_root);
    
    // 验证：无 Root 时回收（环可回收）
    gc(&mut ctx);
    let count_after_drop = get_finalizer_count(&mut ctx);
    assert!(count_after_drop > initial_count,
        "Cycle MUST be collected after Root dropped. Expected > {}, got {}",
        initial_count, count_after_drop);
}

// 辅助函数
fn get_finalizer_count(ctx: &mut mquickjs_rs::Context) -> i32 {
    let result = ctx.eval("TestGc.makeNode().finalizerCount()")
        .expect("get finalizer count");
    // 假设返回字符串形式的数字
    result.parse::<i32>().expect("parse count")
}

fn gc(ctx: &mut mquickjs_rs::Context) {
    unsafe {
        mquickjs_rs::mquickjs_ffi::JS_GC(ctx.ctx);
        mquickjs_rs::mquickjs_ffi::JS_GC(ctx.ctx); // 两次确保完全回收
    }
}
```

**关键改进**：
1. 明确记录 `initial_count`
2. 分三个阶段验证：有 Root → 仍有 Root → 无 Root
3. 每次 GC 后检查计数变化
4. 断言消息清晰，便于调试

### 1.2 整合 JS 测试逻辑

**目标**：将 `basic.js` 的验证逻辑移植到 Rust。

**basic.js 的核心逻辑**：
```javascript
// 创建环
node.setHeld(obj);
node.makeCycle();

// 验证有引用时不回收
const before = node.finalizerCount();
gc();
assert(node.finalizerCount() === before, 'not finalized while reachable');

// 释放引用
node.dropAll();
dropped = null;

// 验证回收
gc();
const after = node2.finalizerCount();
assert(after > before, 'finalized after collection');
```

**Rust 移植**：
- 用 `ctx.eval()` 调用 JS API
- 用 `get_finalizer_count()` 检查计数
- 用 `assert_eq!` / `assert!` 验证

**保留 basic.js**：
- 作为端到端 smoke test
- 验证 JS 侧 API 可用性
- 但不依赖它作为主要验证

---

## 阶段 2：扩展测试覆盖（1-1.5 天）

### 2.1 测试场景矩阵

#### 正向测试（验证回收发生）

```rust
#[test]
fn test_cycle_collected_after_all_roots_dropped() {
    // A ↔ B 环，Root 持有 A
    // 释放 Root → 验证 A 和 B 都回收
    // 预期：finalizer 计数 +2
}

#[test]
fn test_multiple_cycles_collected_independently() {
    // 两个独立环：(A1 ↔ B1) 和 (A2 ↔ B2)
    // Root1 持有 A1，Root2 持有 A2
    // 释放 Root1 → 只回收第一个环（+2）
    // 释放 Root2 → 回收第二个环（再 +2）
}

#[test]
fn test_long_chain_collected() {
    // A → B → C → D → A（长环）
    // Root 持有 A
    // 释放 Root → 整个环回收
    // 预期：finalizer 计数 +4
}

#[test]
fn test_simple_object_collected() {
    // 单个对象，无环
    // Root 持有 → 释放 Root → 回收
    // 验证基本场景
}
```

#### 反向测试（验证不回收）

```rust
#[test]
fn test_root_prevents_collection() {
    // 创建环 + Root
    // 多次 GC（10 次）
    // 验证计数不变（未回收）
}

#[test]
fn test_js_reference_prevents_collection() {
    // 创建环，JS 侧保留引用（globalThis.keepalive）
    // 释放 Root，但 JS 引用存在
    // 验证不回收
    // 释放 JS 引用 → 验证回收
}

#[test]
fn test_partial_root_keeps_cycle_alive() {
    // A ↔ B ↔ C 三角环
    // Root 只持有 A
    // 验证：B 和 C 也不回收（通过 A 可达）
    // 释放 Root → 全部回收
}

#[test]
fn test_multiple_roots_same_object() {
    // Root1 和 Root2 都持有同一个对象
    // 释放 Root1 → 不回收（Root2 仍持有）
    // 释放 Root2 → 回收
}
```

#### 边界测试

```rust
#[test]
fn test_empty_root_drop() {
    // 创建 Root，未形成环，立即 drop
    // 验证不崩溃，计数不变
}

#[test]
fn test_root_outlives_scope() {
    // 编译时测试（compile_fail）
    // Root 不能超出 Scope 生命周期
    // 验证类型系统正确
}

#[test]
#[should_panic(expected = "cross-context Root::new")]
fn test_cross_context_root_rejected() {
    // Context A 创建对象
    // Context B 尝试创建 Root
    // 验证 panic（ctx_id 校验）
}

#[test]
fn test_gc_during_root_creation() {
    // 创建大量对象触发 GC
    // 同时创建 Root
    // 验证 Root 正确保活
}
```

### 2.2 测试辅助工具

**目标**：减少样板代码，提高测试可读性。

```rust
// tests/gc_root_cycle.rs 底部添加
#[cfg(feature = "ridl-extensions")]
mod test_utils {
    use super::*;
    
    pub struct GcTestHarness {
        ctx: mquickjs_rs::Context,
        initial_count: i32,
    }
    
    impl GcTestHarness {
        pub fn new() -> Self {
            mquickjs_rs::ridl_bootstrap!();
            let mut ctx = mquickjs_rs::Context::new(1024 * 1024)
                .expect("create context");
            let initial_count = Self::get_count(&mut ctx);
            Self { ctx, initial_count }
        }
        
        pub fn ctx_mut(&mut self) -> &mut mquickjs_rs::Context {
            &mut self.ctx
        }
        
        pub fn create_node(&mut self) -> String {
            // 返回 JS 变量名
            static COUNTER: AtomicI32 = AtomicI32::new(0);
            let id = COUNTER.fetch_add(1, Ordering::SeqCst);
            let var_name = format!("node{}", id);
            self.ctx.eval(&format!("globalThis.{} = TestGc.makeNode();", var_name))
                .expect("create node");
            var_name
        }
        
        pub fn create_object(&mut self) -> String {
            static COUNTER: AtomicI32 = AtomicI32::new(0);
            let id = COUNTER.fetch_add(1, Ordering::SeqCst);
            let var_name = format!("obj{}", id);
            self.ctx.eval(&format!("globalThis.{} = {{}};", var_name))
                .expect("create object");
            var_name
        }
        
        pub fn make_cycle(&mut self, node: &str, obj: &str) {
            self.ctx.eval(&format!("{}.setHeld({}); {}.back = {};", 
                node, obj, obj, node))
                .expect("make cycle");
        }
        
        pub fn gc(&mut self) {
            unsafe {
                mquickjs_rs::mquickjs_ffi::JS_GC(self.ctx.ctx);
                mquickjs_rs::mquickjs_ffi::JS_GC(self.ctx.ctx);
            }
        }
        
        pub fn assert_not_collected(&mut self, msg: &str) {
            let current = Self::get_count(&mut self.ctx);
            assert_eq!(current, self.initial_count,
                "Unexpected collection: {}", msg);
        }
        
        pub fn assert_collected(&mut self, expected_delta: i32, msg: &str) {
            let current = Self::get_count(&mut self.ctx);
            assert_eq!(current, self.initial_count + expected_delta,
                "Expected {} collections, got {}: {}",
                expected_delta, current - self.initial_count, msg);
        }
        
        pub fn current_count(&mut self) -> i32 {
            Self::get_count(&mut self.ctx)
        }
        
        fn get_count(ctx: &mut mquickjs_rs::Context) -> i32 {
            let result = ctx.eval("TestGc.makeNode().finalizerCount()")
                .expect("get finalizer count");
            result.parse::<i32>().expect("parse count")
        }
    }
}

// 使用示例
#[test]
fn test_example_with_harness() {
    let mut h = test_utils::GcTestHarness::new();
    
    let node = h.create_node();
    let obj = h.create_object();
    h.make_cycle(&node, &obj);
    
    h.gc();
    h.assert_not_collected("cycle with JS ref");
    
    h.ctx_mut().eval(&format!("{} = null; {} = null;", node, obj)).unwrap();
    h.gc();
    h.assert_collected(1, "cycle after ref dropped");
}
```

**关键特性**：
- 自动初始化和记录 initial_count
- 提供高层 API（create_node、make_cycle）
- 封装断言逻辑（assert_not_collected、assert_collected）
- 管理全局变量名（避免冲突）

---

## 阶段 3：TDD 驱动 Traced<T>（2-3 天）

### 3.1 编写失败的测试（Red）

**目标**：先写 Traced<T> 的测试，展示预期行为。

#### 创建新的 RIDL 测试模块

```
tests/global/gc_traced/
└── test_gc_traced/
    ├── Cargo.toml
    ├── build.rs
    ├── src/
    │   ├── lib.rs
    │   ├── test_gc_traced.ridl
    │   ├── traced_node_impl.rs
    │   └── class_impl.rs
    └── tests/
        └── traced.js
```

**RIDL 定义**（使用阶段 0 的新语法）：
```ridl
// test_gc_traced.ridl
class TracedNode {
    opaque {
        held: Option<Traced<Value>>
    }
    
    fn setHeld(v: object) -> void;
    fn clearHeld() -> void;
    fn finalizerCount() -> i32;
}

singleton TestGcTraced {
    fn makeTracedNode() -> TracedNode;
}
```

**Rust 实现（opaque 由 RIDL 生成）**：
```rust
// traced_node_impl.rs
use mquickjs_rs::Traced;

// opaque struct 由 RIDL 自动生成：
// pub struct TracedNodeOpaque {
//     pub held: Option<Traced<Value>>,
// }

static FINALIZER_COUNT: AtomicI32 = AtomicI32::new(0);

impl Drop for TracedNodeOpaque {
    fn drop(&mut self) {
        FINALIZER_COUNT.fetch_add(1, Ordering::SeqCst);
    }
}

impl TracedNodeClass for TracedNodeOpaque {
    fn set_held<'ctx>(&mut self, env: &mut mquickjs_rs::Env<'ctx>, v: Object<'ctx>) {
        let local = env.scope().value(v.as_raw());
        self.held = Some(Traced::new(local));
    }
    
    fn clear_held(&mut self) {
        self.held = None;
    }
    
    fn finalizer_count(&mut self) -> i32 {
        FINALIZER_COUNT.load(Ordering::SeqCst)
    }
}

// gc_mark 由 RIDL 自动生成（基于 opaque 字段）
```
```

#### Rust 测试用例（会失败）

```rust
// tests/gc_traced.rs（新文件）
#[cfg(feature = "ridl-extensions")]
#[test]
#[ignore] // 标记为 ignore，阶段 3 前不运行
fn test_traced_follows_owner_lifetime() {
    mquickjs_rs::ridl_bootstrap!();
    let mut ctx = mquickjs_rs::Context::new(1024 * 1024).unwrap();
    
    let initial_count = get_finalizer_count(&mut ctx);
    
    // 创建 A（TracedNode），B（普通对象）
    ctx.eval("globalThis.a = TestGcTraced.makeTracedNode();").unwrap();
    ctx.eval("globalThis.b = {};").unwrap();
    
    // A.setHeld(B)，B.back = A（形成环）
    ctx.eval("a.setHeld(b); b.back = a;").unwrap();
    
    // 关键：不使用 Root，只依赖 JS 引用
    // 释放 JS 侧的引用
    ctx.eval("a = null; b = null;").unwrap();
    
    // 验证：Traced<T> 不独立 root，环应该被回收
    gc(&mut ctx);
    let after_count = get_finalizer_count(&mut ctx);
    assert!(after_count > initial_count,
        "Traced<T> should NOT root independently. Expected collection, got count {} (initial {})",
        after_count, initial_count);
}

#[cfg(feature = "ridl-extensions")]
#[test]
#[ignore]
fn test_traced_vs_root_behavior_comparison() {
    // 对比测试：展示 Root<T> 和 Traced<T> 的差异
    mquickjs_rs::ridl_bootstrap!();
    let mut ctx = mquickjs_rs::Context::new(1024 * 1024).unwrap();
    
    let initial_count = get_finalizer_count(&mut ctx);
    
    // === 场景 A：Root<T> 阻止回收 ===
    ctx.eval("globalThis.nodeRoot = TestGc.makeNode();").unwrap();
    ctx.eval("globalThis.objRoot = {};").unwrap();
    ctx.eval("nodeRoot.setHeld(objRoot); objRoot.back = nodeRoot;").unwrap();
    
    // nodeRoot 内部用 Root<T>，创建 Rust 侧 Root
    // （当前实现：Node.setHeld 内部创建 Root<T>）
    
    // 释放 JS 引用
    ctx.eval("nodeRoot = null; objRoot = null;").unwrap();
    gc(&mut ctx);
    
    let count_after_root = get_finalizer_count(&mut ctx);
    assert_eq!(count_after_root, initial_count,
        "Root<T> should prevent collection");
    
    // === 场景 B：Traced<T> 允许回收 ===
    ctx.eval("globalThis.nodeTraced = TestGcTraced.makeTracedNode();").unwrap();
    ctx.eval("globalThis.objTraced = {};").unwrap();
    ctx.eval("nodeTraced.setHeld(objTraced); objTraced.back = nodeTraced;").unwrap();
    
    // nodeTraced 内部用 Traced<T>，不创建独立 root
    
    // 释放 JS 引用
    ctx.eval("nodeTraced = null; objTraced = null;").unwrap();
    gc(&mut ctx);
    
    let count_after_traced = get_finalizer_count(&mut ctx);
    assert!(count_after_traced > count_after_root,
        "Traced<T> should allow collection when owner is unreachable");
}
```

**预期结果**：这些测试会失败，因为 `Traced<T>` 尚未实现。

### 3.2 实现 Traced<T>（Green）

#### 3.2.1 mquickjs-rs 侧：定义 Traced<T>

```rust
// deps/mquickjs-rs/src/traced.rs（新文件）
use crate::handles::local::Local;
use crate::mquickjs_ffi;

/// A JSValue held within a JS user class opaque.
///
/// Unlike `Root<T>`, `Traced<T>` does NOT independently root the value.
/// It relies on the owning JS object's reachability:
/// - When the owner is reachable, the GC mark callback will mark this field.
/// - When the owner is unreachable, this field does not prevent collection.
///
/// # Safety Model
///
/// **Invariant**: Traced<T> lifetime ≤ JS object lifetime
///
/// This is guaranteed by:
/// 1. JS object reachable → opaque valid → Traced<T> valid
/// 2. JS object unreachable → GC triggers finalizer → opaque Drop → Traced<T> invalidated
///
/// **Must be used within RIDL user class opaque only.**
///
/// ## Violation Scenarios (MUST avoid)
///
/// ```rust,no_run
/// // ❌ Taking Traced out of opaque
/// let traced = opaque.held.take(); // opaque may be finalized later
/// // traced now holds dangling pointer
///
/// // ❌ Cross-Context usage
/// let traced_from_ctx1 = /* ... */;
/// // Use in ctx2 → undefined behavior
/// ```
///
/// ## Design Decision
///
/// - No runtime checks (performance consideration)
/// - Relies on type system + RIDL constraints for safety
pub struct Traced<T> {
    raw: mquickjs_ffi::JSValue,
    _t: std::marker::PhantomData<T>,
}

impl<T> Traced<T> {
    /// Create a Traced<T> from a Local<T>.
    ///
    /// # Safety
    ///
    /// The caller must ensure this Traced is stored in a RIDL user class opaque
    /// that will be marked by the class's gc_mark callback.
    pub fn new(v: Local<T>) -> Self {
        Self {
            raw: v.as_raw(),
            _t: std::marker::PhantomData,
        }
    }
    
    /// Get the raw JSValue (for use in gc_mark callbacks).
    pub fn as_raw(&self) -> mquickjs_ffi::JSValue {
        self.raw
    }
    
    /// Check if this Traced holds a value (not null/undefined).
    pub fn is_some(&self) -> bool {
        // Simplified: real implementation needs JS_IsNull/JS_IsUndefined
        !self.raw.is_null()
    }
}

// No Drop implementation: Traced does not manage lifetime independently
```

**关键点**：
- `Traced<T>` 只存储 `JSValue`，不调用任何引擎 API
- 无 `Drop` 实现（不像 `Root<T>` 需要从 registry 移除）
- 依赖 RIDL 生成的 gc_mark 回调来标记
- 完整的安全模型文档（不变式 + 违反场景）
```

**关键点**：
- `Traced<T>` 只存储 `JSValue`，不调用任何引擎 API
- 无 `Drop` 实现（不像 `Root<T>` 需要从 registry 移除）
- 依赖 RIDL 生成的 gc_mark 回调来标记

#### 3.2.2 导出 Traced<T>

```rust
// deps/mquickjs-rs/src/lib.rs
pub mod traced;
pub use traced::Traced;
```

#### 3.2.3 RIDL 生成器：自动生成 gc_mark

**基于阶段 0 的 opaque 字段信息自动生成。**

RIDL 生成器已在阶段 0 实现了 `generate_gc_mark()` 函数，该函数：
1. 读取 class 的 `opaque_fields`
2. 筛选出 `Traced<T>` 类型的字段
3. 生成 gc_mark 函数，调用 `mark_value` 标记这些字段

**生成的代码示例**（TracedNode）：
```rust
// 自动生成
impl TracedNodeOpaque {
    pub unsafe extern "C" fn gc_mark(
        _ctx: *mut mquickjs_sys::JSContext,
        _obj: mquickjs_sys::JSValue,
        opaque: *mut std::os::raw::c_void,
        mf: *const mquickjs_sys::JSMarkFunc,
    ) {
        if opaque.is_null() || mf.is_null() {
            return;
        }
        
        let this = &*(opaque as *const Self);
        if let Some(mark_value) = (*mf).mark_value {
            // 自动为每个 Traced 字段生成标记代码
            if let Some(ref field) = this.held {
                mark_value(mf, field.as_raw());
            }
        }
    }
}
```

**C FFI 注册**（自动生成）：
```rust
#[no_mangle]
pub unsafe extern "C" fn js_TracedNode_gc_mark(
    ctx: *mut JSContext,
    obj: JSValue,
    opaque: *mut c_void,
    mf: *const JSMarkFunc,
) {
    TracedNodeOpaque::gc_mark(ctx, obj, opaque, mf);
}
```

**无需手写**：用户只需在 `.ridl` 文件中声明 `opaque { held: Option<Traced<Value>> }`，其余代码全部自动生成。
```

#### 3.2.4 RIDL 模板：注册 gc_mark

```c
// deps/ridl-tool/templates/mquickjs_ridl_register.h.j2
static JSClassDef js_{{ class.name }}_class = {
    .class_name = "{{ class.name }}",
    .finalizer = js_{{ class.name }}_finalizer,
    {% if class.has_traced_fields %}
    .gc_mark = js_{{ class.name }}_gc_mark,
    {% else %}
    .gc_mark = NULL,
    {% endif %}
};
```

**Rust 侧生成**：

```rust
// 生成的代码示例
#[no_mangle]
pub unsafe extern "C" fn js_TracedNode_gc_mark(
    ctx: *mut JSContext,
    obj: JSValue,
    opaque: *mut c_void,
    mf: *const JSMarkFunc,
) {
    DefaultTracedNode::gc_mark(ctx, obj, opaque, mf);
}
```

### 3.3 验证测试通过（Green）

运行之前标记为 `#[ignore]` 的测试：

```bash
# 先构建 RIDL 模块
cargo run -p ridl-builder -- prepare

# 运行 Traced<T> 测试
cargo test --test gc_traced -- --ignored

# 运行所有 GC 测试
cargo test gc_root_cycle gc_traced
```

**预期结果**：
- ✅ `test_traced_follows_owner_lifetime` 通过
- ✅ `test_traced_vs_root_behavior_comparison` 通过
- ✅ 所有阶段 1/2 的测试仍然通过

### 3.4 对比文档

```rust
// tests/gc_root_cycle.rs 或 tests/gc_traced.rs 顶部添加
//! # Root<T> vs Traced<T> 行为对比
//!
//! | 维度 | Root<T> | Traced<T> |
//! |------|---------|-----------|
//! | **用途** | 跨 await 持有、全局队列 | user class opaque 内部字段 |
//! | **是否独立 root** | 是 | 否 |
//! | **对象不可达时** | 阻止回收 | 允许回收 |
//! | **注册机制** | Context-level gc_mark | per-class gc_mark |
//! | **Drop 行为** | 从 RootsRegistry 移除 | 无操作 |
//! | **线程安全** | !Send + !Sync | 同 opaque struct |
//! | **跨 Context** | 校验 ctx_id，panic | 跟随 opaque |
//!
//! ## 使用示例
//!
//! ```rust
//! // Root<T> - 跨 await 持有
//! let root = Root::new(&scope, value);
//! // ... await 或离开 scope ...
//! drop(root); // 显式释放
//!
//! // Traced<T> - opaque 内部字段
//! struct MyClass {
//!     callback: Traced<Function>, // 依赖 MyClass 实例的生命周期
//! }
//! // RIDL 自动生成 gc_mark，无需手动管理
//! ```
```

---

## 技术细节

### GC 验证流程

```
创建对象 → 形成环 → 持有 Root
    ↓
记录 initial_count
    ↓
GC（有 Root）
    ↓
验证 count == initial_count（不回收）
    ↓
释放 Root
    ↓
GC（无 Root）
    ↓
验证 count > initial_count（回收）
```

### Finalizer 计数机制

```rust
// node_impl.rs
static FINALIZER_COUNT: AtomicI32 = AtomicI32::new(0);

impl Drop for DefaultNode {
    fn drop(&mut self) {
        FINALIZER_COUNT.fetch_add(1, Ordering::SeqCst);
    }
}
```

**触发时机**：
1. JS 对象被 mquickjs GC 回收
2. opaque 的 Rust struct 被 finalizer 调用 Drop
3. AtomicI32 计数器增加

**验证方式**：
- 通过 `finalizerCount()` JS API 读取计数
- Rust 测试比较 GC 前后的计数差异

### mquickjs GC 机制

**Context-level gc_mark**（Root<T> 使用）：
```c
JS_SetContextGCMark(ctx, opaque, ctx_gc_mark_fn);
```
- 在 mark roots 阶段调用
- 遍历 RootsRegistry 中的所有 JSValue
- 调用 `mark_value` 标记

**Per-class gc_mark**（Traced<T> 使用）：
```c
JSClassDef {
    .gc_mark = js_MyClass_gc_mark,
}
```
- 在 mark 阶段，对每个该 class 的对象调用
- 遍历 opaque 中的所有 Traced<T> 字段
- 调用 `mark_value` 标记

### 测试覆盖率目标

- **阶段 1**：1 个修复测试 → 验证基本流程正确
- **阶段 2**：+10 个场景测试 → 覆盖正向/反向/边界
- **阶段 3**：+5 个 Traced<T> 测试 → 验证新功能

**总计**：~16 个测试用例

---

## 风险与缓解

### 风险 1：RIDL 生成器改动复杂

**描述**：自动生成 gc_mark 涉及解析 opaque fields、类型判断、代码生成。

**缓解**：
- 阶段 3 前，阶段 1/2 已建立可靠测试
- 先手写 gc_mark 验证逻辑，再自动化生成
- RIDL 生成器有现有基础设施（codegen.rs）

### 风险 2：测试不稳定（GC 时机不确定）

**描述**：GC 可能不立即回收，导致 flaky tests。

**缓解**：
- 双次 `JS_GC()` 确保完全回收
- 使用明确的计数对比，而非时间等待
- 必要时添加 `JS_RunGC()` 强制完整 GC 周期

### 风险 3：Traced<T> 性能开销

**描述**：每个 class 实例都有 gc_mark 回调，可能影响性能。

**缓解**：
- 只对有 Traced 字段的 class 生成 gc_mark
- gc_mark 只遍历 Traced 字段（O(fields)，通常很少）
- 性能测试在功能稳定后添加

### 风险 4：阶段 0 时间估算不确定（新增）

**描述**：RIDL 语法扩展是未知领域，可能遇到意外复杂度（如类型推导、泛型支持）。

**缓解**：
- 先实现最小 MVP（只支持 `Traced<Value>`，不支持泛型）
- 遇到阻塞时，降级到手写 opaque struct + proc-macro 方案
- 时间预留 3-5 天 buffer

### 风险 5：跨模块测试依赖

**描述**：test_gc_traced 依赖 Traced<T> 和 RIDL 生成器同时完成。

**缓解**：
- 阶段 3.1 先写测试，标记为 `#[ignore]`
- 阶段 3.2 分步实现（mquickjs-rs → RIDL generator → template）
- 每步完成后运行部分测试验证

---

## 验收标准

### 阶段 0
- ✅ RIDL parser 支持 `opaque { ... }` 块
- ✅ RIDL AST 存储 opaque 字段类型信息
- ✅ RIDL codegen 生成 opaque struct 定义
- ✅ RIDL codegen 自动生成 gc_mark（针对 Traced 字段）
- ✅ 单元测试（parser + codegen）
- ✅ 集成测试（fixtures/gc_mark.ridl）

### 阶段 1
- ✅ `tests/gc_root_cycle.rs` 中的断言明确且严格
- ✅ 测试验证三个阶段（有 Root 不回收、无 Root 回收）
- ✅ 辅助函数（get_finalizer_count、gc）可复用

### 阶段 2
- ✅ 至少 10 个新测试用例
- ✅ 覆盖正向（回收）、反向（不回收）、边界
- ✅ GcTestHarness 工具可用且减少样板代码
- ✅ 所有测试稳定通过（无 flaky）

### 阶段 3
- ✅ `Traced<T>` 类型定义完整
- ✅ RIDL 生成器自动生成 gc_mark
- ✅ test_gc_traced 模块所有测试通过
- ✅ 对比文档清晰说明 Root<T> vs Traced<T>

### 整体
- ✅ `cargo test` 全部通过
- ✅ 无测试被标记为 `#[ignore]`（阶段 3 完成后）
- ✅ 代码覆盖率（可选）：GC 相关代码 >80%

---

## 交付清单

### 代码
- **阶段 0**：
  - `deps/ridl-tool/src/parser.rs`（扩展 opaque 块解析）
  - `deps/ridl-tool/src/ast.rs`（新增 OpaqueField、Type 定义）
  - `deps/ridl-tool/src/codegen.rs`（生成 opaque struct + gc_mark）
  - `deps/ridl-tool/tests/parser_test.rs`（单元测试）
  - `deps/ridl-tool/tests/fixtures/gc_mark.ridl`（集成测试）
- **阶段 1-2**：
  - `tests/gc_root_cycle.rs`（修复 + 扩展）
- **阶段 3**：
  - `tests/gc_traced.rs`（新增）
  - `tests/global/gc_traced/test_gc_traced/`（新 RIDL 模块）
  - `deps/mquickjs-rs/src/traced.rs`（新增）
  - `deps/mquickjs-rs/src/lib.rs`（导出 Traced）

### 文档
- 本设计文档
- `tests/gc_root_cycle.rs` 模块级文档（Root<T> vs Traced<T> 对比）
- `deps/mquickjs-rs/src/traced.rs` 安全模型文档
- `docs/knowledge/` 中添加相关条目（通过 /post-commit-memory）

### 测试
- 阶段 0：RIDL 工具单元测试 + 集成测试
- 阶段 1：1 个修复测试
- 阶段 2：10+ 个扩展测试
- 阶段 3：5+ 个 Traced<T> 测试
- 总计：~20 个测试用例

---

## 估算

- **阶段 0**：3-5 天（扩展 RIDL 语法 + parser + codegen + 测试）
- **阶段 1**：0.5-1 天（修复断言 + 辅助函数）
- **阶段 2**：1-1.5 天（10 个测试 + GcTestHarness）
- **阶段 3**：2-3 天（Traced<T> 实现 + 测试，RIDL 生成器已在阶段 0 完成）
- **总计**：7-10.5 天

**关键路径**：阶段 0（RIDL 语法扩展）是基础，其他阶段依赖它。

---

## 后续工作（不在本次范围）

- 性能测试（GC overhead benchmark）
- 并发测试（多线程场景，如果支持）
- 文档完善（用户指南、API 文档）
- 更多 RIDL 模块示例（演示 Traced<T> 使用）
- Traced<T> 的高级功能（Option<Traced<T>>、Vec<Traced<T>> 等）
