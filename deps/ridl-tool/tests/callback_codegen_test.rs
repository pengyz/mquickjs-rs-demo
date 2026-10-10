//! 切片 2 渲染测试：RIDL 回调参数 → CallbackHandle 提取 + 具名 callback 的
//! C trampoline。
//!
//! 设计：`docs/superpowers/specs/2026-10-10-callback-bridge-design.md` §3 切片 2。
//!
//! 覆盖：
//! 1. singleton / class 方法上的 callback 参数 → glue 生成
//!    `JS_IsFunction` 校验 + `Local<Function>` 提取 + `register` 注册，
//!    不再残留空 AsyncCallback 闭包；
//! 2. 具名 `callback_def` → 每个生成一个 C 可调用 trampoline
//!    `mqjs_cb_<name>_invoke`（Rust `extern "C"` 实现体进 glue.rs）；
//! 3. 聚合 C 头：trampoline 只声明进 `mquickjs_ridl_api.h`（运行时 C TU
//!    消费；吸取 gc_mark 双头声明教训，`mquickjs_ridl_register.h` 不声明）；
//! 4. trampoline v1 参数类型白名单（bool/i32/f64/string）之外的类型 →
//!    明确的 unsupported 错误；`Optional(callback)` / map 值中的 callback →
//!    明确错误。

use std::fs;
use std::path::PathBuf;

/// 渲染一段 RIDL，返回 (glue.rs, api.rs) 文本。tempdir 在函数内存活到读取完成。
fn generate(ridl: &str) -> (String, String) {
    let tmp = tempfile::tempdir().expect("tempdir");
    let out = tmp.path().join("out");
    fs::create_dir_all(&out).expect("create out dir");

    let parsed = ridl_tool::parser::parse_ridl_file(ridl).expect("parse ridl");
    ridl_tool::generator::generate_module_files(
        &parsed.items,
        parsed.module,
        parsed.mode,
        &out,
        "test_module",
    )
    .expect("generate module files");

    let glue = fs::read_to_string(out.join("glue.rs")).expect("read glue.rs");
    let api = fs::read_to_string(out.join("api.rs")).expect("read api.rs");
    (glue, api)
}

/// 断言 glue 中不再有异步桥 stub 残留（C3 对抗复核发现的 no-op）。
fn assert_no_async_stub(glue: &str) {
    assert!(
        !glue.contains("async_bridge::AsyncCallback"),
        "glue 不应再包含 AsyncCallback stub:\n{glue}"
    );
    assert!(
        !glue.contains("callback will be invoked by async bridge"),
        "glue 不应再包含 async bridge 占位注释:\n{glue}"
    );
}

/// 场景 1a：singleton 方法上的 callback 参数 → handle 提取 + 注册。
#[test]
fn singleton_callback_param_binds_handle_and_drops_stub() {
    let ridl = r#"
        singleton evt {
            fn setHandler(cb: callback (code: i32));
            fn setEmpty(cb: callback ());
        }
    "#;
    let (glue, api) = generate(ridl);

    // 提取链：JS_IsFunction 校验 → Local<Function> → register → handle 传给 impl。
    assert!(
        glue.contains("mquickjs_rs::mquickjs_ffi::JS_IsFunction(ctx, v)"),
        "必须保留 JS_IsFunction 校验:\n{glue}"
    );
    assert!(
        glue.contains("invalid callback argument: cb"),
        "必须保留参数名相关的 type error:\n{glue}"
    );
    assert!(
        glue.contains("try_into_function(&scope)"),
        "必须经 Scope 提取 Local<Function>:\n{glue}"
    );
    assert!(
        glue.contains("h.callbacks().register(&scope, "),
        "必须注册进 CallbackRegistry:\n{glue}"
    );
    assert!(
        glue.contains("let cb: mquickjs_rs::CallbackHandle ="),
        "参数绑定必须是 CallbackHandle:\n{glue}"
    );
    assert_no_async_stub(&glue);

    // api trait：callback 参数 → CallbackHandle；needs_scope → env 注入。
    assert!(
        api.contains("cb: mquickjs_rs::CallbackHandle"),
        "api trait 的 callback 参数应为 CallbackHandle:\n{api}"
    );
    assert!(
        api.contains("env: &mut mquickjs_rs::Env<'ctx>"),
        "callback 参数应使方法进入 needs_scope（env 注入）:\n{api}"
    );
}

/// 场景 1b：class 方法上的 callback 参数（LVGL 主路径）同样生成提取。
#[test]
fn class_callback_param_binds_handle() {
    let ridl = r#"
        class button {
            fn setOnClick(cb: callback (code: i32));
        }
    "#;
    let (glue, api) = generate(ridl);

    assert!(
        glue.contains("h.callbacks().register(&scope, "),
        "class 方法 glue 必须注册回调:\n{glue}"
    );
    assert!(
        glue.contains("let cb: mquickjs_rs::CallbackHandle ="),
        "class 方法参数绑定必须是 CallbackHandle:\n{glue}"
    );
    assert_no_async_stub(&glue);
    assert!(
        api.contains("cb: mquickjs_rs::CallbackHandle"),
        "class trait 参数应为 CallbackHandle:\n{api}"
    );
}

/// 场景 2：具名 callback_def → C trampoline（Rust extern "C" 实现体）。
/// 含 string/f64 参数（堆值）→ 走 GC 安全的 rooted 路径。
#[test]
fn named_callback_def_generates_c_trampoline_with_heap_params() {
    let ridl = r#"
        callback myEvent(text: string, code: i32, flag: bool, ratio: f64);
    "#;
    let (glue, _api) = generate(ridl);

    assert!(
        glue.contains("mqjs_cb_my_event_invoke"),
        "应生成 mqjs_cb_my_event_invoke trampoline:\n{glue}"
    );
    assert!(
        glue.contains("pub unsafe extern \"C\" fn mqjs_cb_my_event_invoke"),
        "trampoline 必须是导出的 extern \"C\" fn:\n{glue}"
    );
    assert!(
        glue.contains("CallbackHandle::from_raw(handle)"),
        "trampoline 必须从 C u32 句柄构造 CallbackHandle:\n{glue}"
    );
    // 参数现场转换（每次调用从 Rust/C 值转换，不缓存 JSValue —— 切片 1 args 契约）。
    assert!(
        glue.contains("JS_NewString(ctx, p_text"),
        "string 参数应经 JS_NewString 现场转换:\n{glue}"
    );
    assert!(
        glue.contains("JS_NewInt32(ctx, p_code"),
        "i32 参数应经 JS_NewInt32 现场转换:\n{glue}"
    );
    assert!(
        glue.contains("js_mkbool(p_flag != 0)"),
        "bool 参数（C int 传入）应经 js_mkbool 现场转换:\n{glue}"
    );
    assert!(
        glue.contains("JS_NewFloat64(ctx, p_ratio"),
        "f64 参数应经 JS_NewFloat64 现场转换:\n{glue}"
    );
    // 堆值参数（string/f64）→ invoke_rooted（GC 安全）。
    assert!(
        glue.contains("invoke_rooted("),
        "含堆值参数的 trampoline 必须走 invoke_rooted:\n{glue}"
    );
    // 异常可见性策略（切片 1 审查遗留）。
    assert!(
        glue.contains("eprintln!"),
        "std 构建下 trampoline 必须让回调失败可见:\n{glue}"
    );
}

/// 场景 2b：纯立即值参数（i32/bool）→ 普通 invoke 路径（不引入 Root 开销）。
#[test]
fn immediate_only_callback_uses_plain_invoke() {
    let ridl = r#"
        callback tick(code: i32, flag: bool);
    "#;
    let (glue, _api) = generate(ridl);

    assert!(
        glue.contains("mqjs_cb_tick_invoke"),
        "应生成 mqjs_cb_tick_invoke:\n{glue}"
    );
    assert!(
        glue.contains("h.callbacks().invoke(cb_handle, &cb_args)"),
        "纯立即值 trampoline 应走普通 invoke:\n{glue}"
    );
    assert!(
        !glue.contains("Root::new"),
        "纯立即值 trampoline 不应引入 Root:\n{glue}"
    );
}

/// 场景 3：聚合 C 头 —— trampoline 只声明进 api.h（运行时 C TU 消费），
/// register.h（ROM host tool）不声明（gc_mark 双头声明教训）。
#[test]
fn trampoline_declared_only_in_api_h() {
    let tempdir = tempfile::tempdir().unwrap();
    let out_dir = tempdir.path().to_path_buf();

    let module = ridl_tool::plan::RidlModule {
        crate_name: "m1".to_string(),
        name: "m1".to_string(),
        crate_dir: PathBuf::from("."),
        ridl_files: vec![PathBuf::from("tests/fixtures_callback_codegen.ridl")],
    };
    let plan = ridl_tool::plan::RidlPlan {
        schema_version: 0,
        cargo_toml: PathBuf::from("Cargo.toml"),
        modules: vec![module],
        generated: ridl_tool::plan::GeneratedPaths {
            out_dir: out_dir.clone(),
            mquickjs_ridl_register_h: out_dir.join("mquickjs_ridl_register.h"),
            mquickjs_ridl_module_class_ids_h: out_dir.join("mquickjs_ridl_module_class_ids.h"),
            mqjs_ridl_user_class_ids_h: out_dir.join("mqjs_ridl_user_class_ids.h"),
            ridl_class_id_rs: out_dir.join("ridl_class_id.rs"),
        },
        inputs: vec![],
    };

    ridl_tool::generator::generate_aggregate_consolidated(&plan, &out_dir).unwrap();

    let api_h = fs::read_to_string(out_dir.join("mquickjs_ridl_api.h")).unwrap();
    let register_h = fs::read_to_string(out_dir.join("mquickjs_ridl_register.h")).unwrap();

    let decl = "void mqjs_cb_my_event_invoke(JSContext *ctx, uint32_t handle, const char *p_text, int32_t p_code);";
    assert!(
        api_h.contains(decl),
        "api.h 必须声明 trampoline（C ABI 签名）:\n期望: {decl}\n实际:\n{api_h}"
    );
    assert!(
        !register_h.contains("mqjs_cb_my_event_invoke"),
        "register.h 不应声明 trampoline（gc_mark 双头声明教训）:\n{register_h}"
    );
}

/// 场景 4a：trampoline 参数类型白名单之外的类型 → 明确 unsupported 错误。
#[test]
fn unsupported_callback_param_type_is_rejected() {
    let ridl = r#"
        singleton evt {
            fn set(cb: callback (items: array<object>));
        }
    "#;
    let err = generate_expect_err(ridl);
    let msg = format!("{err}");
    assert!(
        msg.contains("unsupported"),
        "错误信息应包含 unsupported:\n{msg}"
    );
    assert!(
        msg.contains("items"),
        "错误信息应指明参数名:\n{msg}"
    );
}

/// 场景 4b：具名 callback_def 的 unsupported 参数同样在生成期报错。
#[test]
fn unsupported_named_callback_param_type_is_rejected() {
    let ridl = "callback bad(cb2: callback (code: i32));";
    let err = generate_expect_err(ridl);
    let msg = format!("{err}");
    assert!(
        msg.contains("unsupported") && msg.contains("bad"),
        "错误信息应包含 unsupported 与回调名:\n{msg}"
    );
}

/// 场景 4c：Optional(callback) 参数 → 明确错误（v1 不支持可缺省回调）。
#[test]
fn optional_callback_param_is_rejected() {
    let ridl = r#"
        singleton evt {
            fn set(cb: callback (code: i32)?);
        }
    "#;
    let err = generate_expect_err(ridl);
    let msg = format!("{err}");
    assert!(
        msg.contains("optional callback"),
        "错误信息应说明 optional callback 不支持:\n{msg}"
    );
}

/// 场景 4d：map 值中的 callback → 明确错误（map 提取上下文没有 ctx 句柄）。
#[test]
fn callback_as_map_value_is_rejected() {
    let ridl = r#"
        singleton evt {
            fn set(m: map<string, callback (code: i32)>);
        }
    "#;
    let err = generate_expect_err(ridl);
    let msg = format!("{err}");
    assert!(
        msg.contains("map") && msg.contains("callback"),
        "错误信息应说明 map 不支持 callback 值:\n{msg}"
    );
}

fn generate_expect_err(ridl: &str) -> Box<dyn std::error::Error> {
    let tmp = tempfile::tempdir().expect("tempdir");
    let out = tmp.path().join("out");
    fs::create_dir_all(&out).expect("create out dir");
    let parsed = ridl_tool::parser::parse_ridl_file(ridl).expect("parse ridl");
    match ridl_tool::generator::generate_module_files(
        &parsed.items,
        parsed.module,
        parsed.mode,
        &out,
        "test_module",
    ) {
        Ok(()) => panic!("expected generation to fail for:\n{ridl}"),
        Err(e) => e,
    }
}
