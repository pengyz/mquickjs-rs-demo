//! OpenVela adapter for mquickjs-rs — dual-mode (std / no_std).
//!
//! 本 crate 是 openvela `js` builtin 的 **RIDL 叶子应用**（M1-R+）：
//! JS Context 的创建完全走 Rust 侧（mquickjs 的 Context 扩展状态挂在
//! `JSContext` user_data 上，只有 `mquickjs_rs::Context::new` 会设置它
//! ——C 侧 `JS_NewContext` 创建的 context 无法承载 RIDL 胶水的分派）。
//!
//! # 运行模式（Phase 3.4，cfg 随目标切换）
//!
//! - `target_os = "linux"`（openvela sim）：std 模式。宿主 Linux 进程
//!   （glibc 动态链接），std 完全可用——无需 GlobalAlloc 桥 /
//!   panic_handler / eh_personality 桩。
//! - 其他目标（QEMU aarch64 裸机 `aarch64-unknown-none`）：no_std 模式。
//!   提供 bare-metal 三件套（`#[global_allocator]` NuttX malloc/free 桥、
//!   `#[panic_handler]`、`rust_eh_personality` 桩，形态源自 commit
//!   d945524 的 Phase 2a adapter），console 输出由 stdlib 的 no_std 模式
//!   经 FFI printf 落到 NuttX 控制台。
//!
//! 两种模式共用同一份业务代码（context trio + RIDL bootstrap）；
//! 仅进程级原语按模式分派（`std::sync::Once` / `thread_local!` ↔
//! AtomicBool / 单线程静态——单线程假设与 mquickjs-rs 的 `TlsCell`
//! 同构：`Context` 持裸引擎指针（`!Send`），js builtin 由单一 NSH
//! 线程驱动每个 context 的 create → eval → free 全程）。
//!
//! # C 导出面
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
//! # 链接模式
//!
//! `MQJS_ENGINE_LINK=external`（由本 crate 的 .cargo/config.toml 固定设置）
//! ——引擎 C 对象由 openvela 源码级集成提供（含 ridl 变体 stdlib 的
//! `mqjs_stdlib_impl.c` 与聚合寄存器 `mquickjs_ridl_register.c`），本
//! staticlib 只含 Rust 对象，`JS_*`/`js_stdlib`/`JS_RIDL_StdlibInit` 等
//! 符号留待镜像链接期解析。

#![cfg_attr(not(target_os = "linux"), no_std)]
#![cfg_attr(not(target_os = "linux"), allow(internal_features))] // rust_eh_personality stub (rust#56152 workaround)

#[cfg(not(target_os = "linux"))]
extern crate alloc;
#[cfg(not(target_os = "linux"))]
use alloc::vec::Vec;

use core::ffi::c_void;

// ---- 裸机运行时三件套（仅 no_std；std 模式由宿主 libc/std 提供） ----

#[cfg(not(target_os = "linux"))]
mod bare_metal {
    use core::alloc::{GlobalAlloc, Layout};
    use core::panic::PanicInfo;
    use core::ptr;

    // libc FFI（最终镜像内解析到 NuttX libc.a；flat 链接下 printf 的
    // stdout 与 js_main 的 printf 同一 NSH 控制台）。
    unsafe extern "C" {
        fn malloc(size: usize) -> *mut u8;
        fn free(ptr: *mut u8);
        fn printf(fmt: *const u8, ...) -> i32;
        fn abort() -> !;
    }

    // GlobalAlloc 桥：NuttX malloc/free。
    pub struct NuttXAlloc;

    unsafe impl GlobalAlloc for NuttXAlloc {
        unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
            // debug assert：当前依赖图只需 <=8 对齐；NuttX malloc 保证 8。
            // 更高对齐的未来依赖需要 posix_memalign——debug 构建大声失败，
            // release 构建返回 null（→ 分配失败 → panic → abort），
            // 决不让低对齐内存被静默使用。
            debug_assert!(layout.align() <= 8, "alignment > 8 not supported by bridge");
            if layout.align() > 8 {
                return ptr::null_mut();
            }
            unsafe { malloc(layout.size()) }
        }
        unsafe fn dealloc(&self, ptr: *mut u8, _layout: Layout) {
            unsafe { free(ptr) }
        }
    }

    #[global_allocator]
    static ALLOCATOR: NuttXAlloc = NuttXAlloc;

    // panic handler（printf + abort；rust#56152 的 personality 桩在下方）。
    // panic=abort 下 panic_handler 仍是必需的 lang item：abort 策略只在
    // panic 触发后的收尾阶段换用 abort，入口仍是本函数。
    #[panic_handler]
    fn panic(info: &PanicInfo) -> ! {
        // Panic 消息的提取依赖 fmt 机制（no_std 下不可用），只上报位置。
        // 文件路径是 &str（非 NUL 结尾），拷进定长栈缓冲、截断、补 NUL
        // 后交给 printf %s。panic 路径零分配。
        const FILE_BUF_LEN: usize = 128;
        let mut file_buf = [0u8; FILE_BUF_LEN];
        if let Some(loc) = info.location() {
            let file = loc.file().as_bytes();
            let n = file.len().min(FILE_BUF_LEN - 1);
            file_buf[..n].copy_from_slice(&file[..n]);
            file_buf[n] = 0;
            unsafe {
                printf(
                    b"RUST PANIC at %s:%u:%u\n\0".as_ptr(),
                    file_buf.as_ptr(),
                    loc.line(),
                    loc.column(),
                );
            }
        } else {
            unsafe {
                printf(b"RUST PANIC\0".as_ptr());
            }
        }
        unsafe { abort() }
    }

    // rust_eh_personality 桩（rust#56152：预编译 alloc/core 的 unwind 形态
    // 引用该符号；panic=abort 从不调用它，但 flat 镜像链接期必须可解析）。
    #[unsafe(no_mangle)]
    pub extern "C" fn rust_eh_personality() {}
}

// ---- 进程级原语（模式分派） ----

/// Process-level RIDL state (module keepalive + symbol keepalive tables),
/// initialized once on the first `mqjs_rs_ridl_context_new` via
/// [`mquickjs_rs::ridl_bootstrap`] (which includes OUT_DIR/ridl_bootstrap.rs).
#[cfg(target_os = "linux")]
static RIDL_BOOTSTRAP: std::sync::Once = std::sync::Once::new();

#[cfg(not(target_os = "linux"))]
static RIDL_BOOTSTRAP_DONE: core::sync::atomic::AtomicBool =
    core::sync::atomic::AtomicBool::new(false);

#[cfg(target_os = "linux")]
fn ridl_bootstrap_once() {
    RIDL_BOOTSTRAP.call_once(|| {
        mquickjs_rs::ridl_bootstrap!();
    });
}

#[cfg(not(target_os = "linux"))]
fn ridl_bootstrap_once() {
    use core::sync::atomic::Ordering;
    // 单线程驱动假设（见模块文档）：swap 语义在此已足够；
    // panic=abort 下无"失败重试"路径。
    if !RIDL_BOOTSTRAP_DONE.swap(true, Ordering::AcqRel) {
        mquickjs_rs::ridl_bootstrap!();
    }
}

// Live RIDL contexts keyed by the raw `JSContext` pointer handed to C.
//
// Thread-local on purpose (std mode): `Context` holds raw engine pointers
// (not `Send`), and the openvela `js` builtin drives each context from a
// single NSH thread end to end (create → eval → free). The engine heap block
// is owned by the `Context` (8-byte aligned `Vec<u64>` via the allocator =
// glibc malloc on sim / NuttX malloc on bare metal) and drops with it, so C
// never mallocs/frees the heap itself — the alloc/free pair stays inside Rust.
//
// 裸机没有 `thread_local!`：以"单线程假设"的 `UnsafeCell` 容器替代
// （与 mquickjs-rs 的 `TlsCell` 同构），`.with(...)` 调用面保持一致，
// 业务代码两种模式共用。
#[cfg(target_os = "linux")]
thread_local! {
    static CONTEXTS: core::cell::RefCell<Vec<(usize, mquickjs_rs::Context)>> =
        const { core::cell::RefCell::new(Vec::new()) };
}

#[cfg(not(target_os = "linux"))]
struct CtxCell(core::cell::UnsafeCell<core::cell::RefCell<Vec<(usize, mquickjs_rs::Context)>>>);

#[cfg(not(target_os = "linux"))]
unsafe impl Sync for CtxCell {}

#[cfg(not(target_os = "linux"))]
impl CtxCell {
    const fn new() -> Self {
        Self(core::cell::UnsafeCell::new(core::cell::RefCell::new(Vec::new())))
    }

    fn with<R>(&self, f: impl FnOnce(&mut Vec<(usize, mquickjs_rs::Context)>) -> R) -> R {
        // Safety: 单线程访问（见模块文档）。
        unsafe {
            let mut guard = (*self.0.get()).borrow_mut();
            f(&mut guard)
        }
    }
}

#[cfg(not(target_os = "linux"))]
static CONTEXTS: CtxCell = CtxCell::new();

/// 统一两种模式的 CONTEXTS 访问面：`f` 拿到 `&mut Vec<(key, Context)>`。
#[cfg(target_os = "linux")]
fn with_contexts<R>(f: impl FnOnce(&mut Vec<(usize, mquickjs_rs::Context)>) -> R) -> R {
    CONTEXTS.with(|cell| f(&mut cell.borrow_mut()))
}

#[cfg(not(target_os = "linux"))]
fn with_contexts<R>(f: impl FnOnce(&mut Vec<(usize, mquickjs_rs::Context)>) -> R) -> R {
    CONTEXTS.with(f)
}

/// Returns a static NUL-terminated version string. Never freed.
#[unsafe(no_mangle)]
pub extern "C" fn mqjs_rs_version() -> *const u8 {
    // 文案区分运行模式：rs_probe 语料仅校验非空字符串，
    // 两模式输出可从哨兵日志直接分辨。
    #[cfg(target_os = "linux")]
    return b"mquickjs-rs std (openvela port, ridl stdlib)\0".as_ptr();
    #[cfg(not(target_os = "linux"))]
    return b"mquickjs-rs no-std (openvela port, ridl stdlib)\0".as_ptr();
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
    ridl_bootstrap_once();

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
    with_contexts(|ctxs| ctxs.push((key, ctx)));
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

    // The &mut Context only exists inside the CONTEXTS borrow, so the
    // eval (and any RIDL callbacks it triggers on this same thread) runs
    // entirely within `with`.
    with_contexts(|ctxs| -> i32 {
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
    with_contexts(|ctxs| {
        if let Some(pos) = ctxs.iter().position(|(k, _)| *k == key) {
            let (_, ctx) = ctxs.swap_remove(pos);
            drop(ctx);
        }
    });
}

/// Generated per-context RIDL support (CtxExt + `ridl_context_init`).
/// Content comes from this app's aggregate via build.rs -> OUT_DIR.
#[allow(dead_code)] // generated artifacts carry unused slot-index consts
mod mqjs_ridl_ctx_ext {
    include!(concat!(env!("OUT_DIR"), "/ridl_context_ext.rs"));
}
