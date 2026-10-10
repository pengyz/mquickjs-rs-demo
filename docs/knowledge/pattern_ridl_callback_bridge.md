# pattern: RIDL 同步回调桥（many-shot callback）

**类型**：pattern（代码约定 / 集成模式）
**日期**：2026-10-10
**实现**：mquickjs-rs `callbacks.rs`（CallbackRegistry）+ ridl-tool 切片 2 codegen

## 形状

- **JS → Rust**：方法参数 `cb: callback(code: i32)` → glue 校验
  `JS_IsFunction` → `Local<Function>` → `ctx.callbacks().register()` →
  impl 收到 `CallbackHandle(u32)`（可存可传，LVGL user_data 形状）。
- **C/Rust → JS**：`mqjs_cb_<name>_invoke(ctx, handle, ...)` trampoline
  （api.h 声明 / glue 实现）→ `ContextToken::from_js_ctx(ctx).callbacks()`
  → `invoke_rooted`（堆参数经 Root 持有，GC 压缩安全）→ 同步 `JS_Call`。
- **异常策略**：invoke 返回 `Err(JsException(msg))` 且保证 ctx 无残留异常
  （含嵌套 throw——toString 用户代码再抛的场景）；trampoline 打印上报。
- **与 async bridge 的边界**：async = fire-and-forget 完成通知（FnOnce+Send）；
  回调桥 = 同线程 many-shot 同步调用（事件语义）。两者正交，勿混用。

## v1 限制

- 参数白名单：bool/i32/f64/string/Optional(String)（NULL → JS null，
  error-first 约定）；其它类型生成期报 unsupported。
- 回调无返回值（grammar callback_def 本就无返回类型）。
- Optional(callback) 拒绝（Option<CallbackHandle> 语义延后）。

## 踩坑记录

- **grammar keyword 表漏 `callback`**：可空形式被 custom_type 抢占解析成
  `Custom(全文)`——新增语法关键字必须同步进 keyword 表。
- **parse_type 缺 callback_type 早返回分支**：来自 parse_nullable_type 的
  独立 pair 落入 `Custom(pair.as_str())` 兜底——mirror traced_type 的先例。
- 测试忽略 generate Err + 直接 read 产物 ⇒ NotFound——失败被伪装成文件
  缺失；测试必须 unwrap 生成 Result。

**关联**：[[architecture_gc_root_traced_unified_tracing]]（JSGCRef 持有）
