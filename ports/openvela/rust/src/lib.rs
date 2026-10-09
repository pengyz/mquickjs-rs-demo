//! OpenVela adapter for mquickjs-rs (std mode).
//!
//! openvela sim 是宿主 Linux 进程（glibc 动态链接），Rust std 在此环境
//! 完全可用——无需 GlobalAlloc 桥、panic_handler 或 eh_personality 桩
//! （这些是裸机 MCU no_std 变体的需求，变体保留在 git 历史中，
//! 见 commit d945524 的 lib.rs）。
//!
//! 本 crate 是 openvela sim 的 **RIDL 叶子应用**（M1-R+）：JS Context 的
//! 创建完全走 Rust 侧（mquickjs 的 Context 扩展状态挂在 `JSContext`
//! user_data 上，只有 `mquickjs_rs::Context::new` 会设置它——C 侧
//! `JS_NewContext` 创建的 context 无法承载 RIDL 胶水的分派），因此对外
//! 提供 C 导出面：
//!
//! - `mqjs_rs_version()` — 版本字符串（保留，rs_probe 语料）
//! - `mqjs_rs_self_test()` — 无 RIDL 的 Context/eval 自检（保留）
//! - `mqjs_rs_ridl_context_new(heap_bytes)` — 创建含 RIDL stdlib 的
//!   Context（进程级 bootstrap 一次 + ridl_context_init + C 侧
//!   `JS_RIDL_StdlibInit`），返回不透明句柄（即 `JSContext*`）
//! - `mqjs_rs_ridl_eval(ctx, buf, len, err_buf, cap)` — 在该 Context 中
//!   eval；异常文本复制进调用方缓冲（无跨 FFI 分配）
//! - `mqjs_rs_ridl_context_free(ctx)` — 销毁 Context（引擎堆块由 Rust 侧
//!   持有并随 `Context::drop` 释放）
//!
//! 构建模式：`MQJS_ENGINE_LINK=external`（由本 crate 的 .cargo/config.toml
//! 固定设置）——引擎 C 对象由 openvela 源码级集成提供（含 ridl 变体
//! stdlib 的 `mqjs_stdlib_impl.c` 与聚合寄存器 `mquickjs_ridl_register.c`），
//! 本 staticlib 只含 Rust 对象，`JS_*`/`js_stdlib`/`JS_RIDL_StdlibInit`
//! 等符号留待镜像链接期解析。
//!
//! std 注意事项：mquickjs-rs 以默认 features（std + ridl-extensions）构建，
//! RIDL stdlib（console）依赖的 std I/O（`println!` → stdout）在 sim 上
//! 原生工作（stdout 即 NSH 控制台）。

use core::ffi::c_void;

/// Generated per-context RIDL support (CtxExt + `ridl_context_init`).
/// Content comes from this app's aggregate via build.rs -> OUT_DIR.
#[allow(dead_code)] // generated artifacts carry unused slot-index consts
mod mqjs_ridl_ctx_ext {
    include!(concat!(env!("OUT_DIR"), "/ridl_context_ext.rs"));
}

/// Process-level RIDL state (module keepalive + symbol keepalive tables),
/// initialized once on the first `mqjs_rs_ridl_context_new` via
/// [`mquickjs_rs::ridl_bootstrap`] (which includes OUT_DIR/ridl_bootstrap.rs).
static RIDL_BOOTSTRAP: std::sync::Once = std::sync::Once::new();

// Live RIDL contexts keyed by the raw `JSContext` pointer handed to C.
//
// Thread-local on purpose: `Context` holds raw engine pointers (not `Send`),
// and the openvela `js` builtin drives each context from a single NSH thread
// end to end (create → eval → free). The engine heap block is owned by the
// `Context` (8-byte aligned `Vec<u64>` via the std allocator = glibc malloc
// on sim) and drops with it, so C never mallocs/frees the heap itself — the
// alloc/free pair stays inside Rust.
thread_local! {
    static CONTEXTS: core::cell::RefCell<Vec<(usize, mquickjs_rs::Context)>> =
        const { core::cell::RefCell::new(Vec::new()) };
}

/// Returns a static NUL-terminated version string. Never freed.
#[unsafe(no_mangle)]
pub extern "C" fn mqjs_rs_version() -> *const u8 {
    b"mquickjs-rs std (openvela port, ridl stdlib)\0".as_ptr()
}

/// End-to-end self test WITHOUT RIDL: create a plain Context (2 MiB heap),
/// eval "1+1", verify result == "2", drop. Returns 0 on success, 1 on failure.
#[unsafe(no_mangle)]
pub extern "C" fn mqjs_rs_self_test() -> i32 {
    match mquickjs_rs::Context::new(2 * 1024 * 1024) {
        Ok(mut ctx) => match ctx.eval("1+1") {
            Ok(result) => {
                if result.trim() == "2" {
                    0
                } else {
                    1
                }
            }
            Err(_) => 1,
        },
        Err(_) => 1,
    }
}

/// Create a JSContext with the RIDL stdlib wired (console singleton et al).
///
/// Mirrors apps/demo `Context::new` + `ridl_context_init` + `JS_RIDL_StdlibInit`:
/// 1. process-level bootstrap once (module + symbol keepalive),
/// 2. `Context::new(heap_bytes)` — sets the `JSContext` user_data that the
///    generated glue dispatches through,
/// 3. `ridl_context_init(ctx)` — allocates CtxExt and fills the ctx slots,
/// 4. `JS_RIDL_StdlibInit(ctx)` — C-side stdlib normalization (implemented by
///    the aggregate's `mquickjs_ridl_register.c` compiled into the image).
///
/// Returns an opaque handle (= `JSContext*`) or NULL on failure.
#[unsafe(no_mangle)]
pub extern "C" fn mqjs_rs_ridl_context_new(heap_bytes: usize) -> *mut c_void {
    RIDL_BOOTSTRAP.call_once(|| {
        mquickjs_rs::ridl_bootstrap!();
    });

    let ctx = match mquickjs_rs::Context::new(heap_bytes) {
        Ok(ctx) => ctx,
        Err(_) => return core::ptr::null_mut(),
    };

    unsafe {
        mqjs_ridl_ctx_ext::ridl_context_init(ctx.ctx);
    }

    // C side (image): stdlib normalization. 0 on success, -1 on exception.
    let rc = unsafe { mquickjs_rs::mquickjs_ffi::JS_RIDL_StdlibInit(ctx.ctx) };
    if rc != 0 {
        // Not registered yet — plain drop runs the full teardown chain.
        drop(ctx);
        return core::ptr::null_mut();
    }

    let key = ctx.ctx as usize;
    CONTEXTS.with(|cell| cell.borrow_mut().push((key, ctx)));
    key as *mut c_void
}

/// Eval `script[0..len]` inside a context created by
/// [`mqjs_rs_ridl_context_new`]. On JS exception, copies the exception text
/// (NUL-terminated, truncated to fit) into `err_buf` and returns 1;
/// returns 0 on success, -1 on usage errors (unknown ctx / invalid UTF-8).
///
/// No cross-FFI allocation: the caller owns `err_buf` (`err_cap` includes the
/// NUL byte; ignored when `err_buf` is NULL or `err_cap` == 0).
#[unsafe(no_mangle)]
pub extern "C" fn mqjs_rs_ridl_eval(
    ctx_opaque: *mut c_void,
    script: *const u8,
    len: usize,
    err_buf: *mut u8,
    err_cap: usize,
) -> i32 {
    let report = |msg: &str| {
        if !err_buf.is_null() && err_cap > 0 {
            let bytes = msg.as_bytes();
            let n = bytes.len().min(err_cap - 1);
            unsafe {
                core::ptr::copy_nonoverlapping(bytes.as_ptr(), err_buf, n);
                *err_buf.add(n) = 0;
            }
        }
    };

    if script.is_null() && len > 0 {
        report("null script buffer");
        return -1;
    }

    let code = match core::str::from_utf8(unsafe {
        if len == 0 {
            &[]
        } else {
            core::slice::from_raw_parts(script, len)
        }
    }) {
        Ok(code) => code,
        Err(_) => {
            report("script is not valid UTF-8");
            return -1;
        }
    };

    // The &mut Context only exists inside the thread-local borrow, so the
    // eval (and any RIDL callbacks it triggers on this same thread) runs
    // entirely within `with`.
    CONTEXTS.with(|cell| -> i32 {
        let mut ctxs = cell.borrow_mut();
        let Some(ctx) = ctxs
            .iter_mut()
            .find(|(k, _)| *k == ctx_opaque as usize)
            .map(|(_, ctx)| ctx)
        else {
            report("unknown context handle");
            return -1;
        };

        match ctx.eval(code) {
            Ok(_) => 0,
            Err(err) => {
                report(&err);
                1
            }
        }
    })
}

/// Destroy a context created by [`mqjs_rs_ridl_context_new`]. Dropping the
/// `Context` runs `JS_FreeContext` (which also drops the user_data Arc and,
/// through it, the RIDL CtxExt slots) and releases the engine heap block.
#[unsafe(no_mangle)]
pub extern "C" fn mqjs_rs_ridl_context_free(ctx_opaque: *mut c_void) {
    if ctx_opaque.is_null() {
        return;
    }
    let key = ctx_opaque as usize;
    CONTEXTS.with(|cell| {
        let mut ctxs = cell.borrow_mut();
        if let Some(pos) = ctxs.iter().position(|(k, _)| *k == key) {
            let (_, ctx) = ctxs.swap_remove(pos);
            drop(ctx);
        }
    });
}
