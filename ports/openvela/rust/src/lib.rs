//! OpenVela adapter for mquickjs-rs (no_std core into NuttX sim image).
//!
//! Provides the single `#[global_allocator]` and `#[panic_handler]` required
//! by the Rust alloc/core in the final image, plus a minimal C export surface
//! for the M1-R acceptance probe (see
//! `docs/planning/2026/2026-10-09-openvela-phase2a.md`, D1/D5).
//!
//! 构建模式：`MQJS_ENGINE_LINK=external`（由本 crate 的 `.cargo/config.toml`
//! 固定设置）——引擎 C 对象由 openvela 源码级集成提供，本 staticlib 只含
//! Rust 对象，`JS_*`/`js_stdlib` 等符号留待镜像链接期解析。
#![no_std]
#![allow(internal_features)] // rust_eh_personality stub (rust#56152 workaround)

use core::alloc::{GlobalAlloc, Layout};
use core::ptr;

// ---- libc FFI (resolved to NuttX libc.a in the final image) ----
unsafe extern "C" {
    fn malloc(size: usize) -> *mut u8;
    fn free(ptr: *mut u8);
    fn printf(fmt: *const u8, ...) -> i32;
    fn abort() -> !;
}

// ---- GlobalAlloc bridge: NuttX malloc/free ----
struct NuttXAlloc;

unsafe impl GlobalAlloc for NuttXAlloc {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        // debug assert: current dependency graph needs <= 8 alignment;
        // NuttX malloc guarantees 8. Future crates with higher alignment
        // need posix_memalign — fail loudly in debug builds, and hand back
        // null (→ allocation error → panic → abort) in release builds so
        // under-aligned memory is never silently used.
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

// ---- panic handler (printf + abort; rust#56152 personality stub below) ----
#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    // Panic message extraction needs fmt machinery we don't have in no_std —
    // report location only. The file path is a Rust &str (not NUL-terminated),
    // so copy it into a fixed stack buffer, truncate if needed, and
    // NUL-terminate for printf %s. No allocation on the panic path.
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
            printf(b"RUST PANIC\n\0".as_ptr());
        }
    }
    unsafe { abort() }
}

// rust_eh_personality stub (rust#56152: hosted targets build alloc with
// unwind; panic=abort never calls it, but the symbol must resolve)
#[unsafe(no_mangle)]
pub extern "C" fn rust_eh_personality() {}

// ---- C export surface (M1-R acceptance probe) ----

/// Returns a static NUL-terminated version string. Never freed.
#[unsafe(no_mangle)]
pub extern "C" fn mqjs_rs_version() -> *const u8 {
    b"mquickjs-rs no-std (openvela port)\0".as_ptr()
}

/// End-to-end self test: create a Context (2 MiB via GlobalAlloc -> malloc),
/// eval "1+1", verify result == "2", drop. Returns 0 on success, 1 on failure.
///
/// `Context::new(memory_capacity)` 分配内部 8 字节对齐堆块（Vec<u64> ->
/// GlobalAlloc -> NuttX malloc），`Context::eval` 返回完成值的字符串形式
/// （`Result<String, String>`）；`Context` 的 `Drop` 负责 `JS_FreeContext`
/// 并随作用域结束自动释放 2 MiB 块。
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
