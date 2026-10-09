//! RIDL stdlib: Rust-side implementations.
//!
//! NOTE: This file intentionally starts minimal.
//! We are currently focusing on getting `console.log(content: string)` working
//! end-to-end via the build-time stdlib injection mechanism.
//!
//! 运行模式分派（feature 卫生，openvela QEMU aarch64 3.4，见 ../Cargo.toml）：
//!
//! - `std`（默认）：`print!`/`eprint!` → 进程 stdout/stderr（NuttX sim 上
//!   即 NSH 控制台）；
//! - `no-std`：FFI `printf` → 镜像 libc 的 stdout。NuttX flat 链接下
//!   stdout 与 stderr 是同一控制台设备，`is_err` 仅保留签名对称
//!   （不再有 fd 语义分离）。绑定一致性：printf/malloc/free 全部解析到
//!   NuttX 侧 libc，与 C 轨（js_main 的 printf）同源。

#[cfg(feature = "no-std")]
use alloc::vec::Vec;
#[cfg(not(feature = "no-std"))]
use std::vec::Vec;

#[cfg(feature = "no-std")]
use core::ffi::CStr;
#[cfg(not(feature = "no-std"))]
use std::ffi::CStr;

use mquickjs_rs::mquickjs_ffi::{JSContext, JSValue};

// ---- 输出端（模式分派） ----

// no_std 端：镜像 libc 的 printf（flat 链接解析到 NuttX libc）。
// 只接受 NUL 结尾的字节串指针；格式串常量内置 `\0`。
#[cfg(feature = "no-std")]
unsafe extern "C" {
    fn printf(fmt: *const u8, ...) -> i32;
}

// 打印一段 NUL 结尾的 C 字符串（printf("%s")）。
//
// Safety: `cstr` 必须指向合法的 NUL 结尾缓冲（调用方保证：
// 引擎 `JS_ToCString` 返回值或静态字面量）。
#[cfg(feature = "no-std")]
unsafe fn pf_cstr(cstr: *const u8) {
    unsafe { printf(b"%s\0".as_ptr(), cstr) };
}

// no_std（NuttX flat 链接）下 stdout/stderr 为同一控制台设备，is_err 无
// fd 语义可分派，参数仅为两模式签名对称保留。
#[cfg_attr(feature = "no-std", allow(unused_variables))]
fn print_js_values(ctx: *mut JSContext, args: &[JSValue], is_err: bool) {
    for (i, v) in args.iter().copied().enumerate() {
        if i != 0 {
            #[cfg(feature = "no-std")]
            unsafe {
                pf_cstr(b" \0".as_ptr());
            }
            #[cfg(not(feature = "no-std"))]
            if is_err {
                eprint!(" ");
            } else {
                print!(" ");
            }
        }

        let mut buf = mquickjs_rs::mquickjs_ffi::JSCStringBuf { buf: [0u8; 5] };
        let ptr = unsafe { mquickjs_rs::mquickjs_ffi::JS_ToCString(ctx, v, &mut buf as *mut _) };
        if ptr.is_null() {
            #[cfg(feature = "no-std")]
            unsafe {
                pf_cstr(b"[toString failed]\0".as_ptr());
            }
            #[cfg(not(feature = "no-std"))]
            if is_err {
                eprint!("[toString failed]");
            } else {
                print!("[toString failed]");
            }
            continue;
        }

        // 与 std 路径同语义：先按 UTF-8 校验，非法时打印占位文本。
        // （校验只在 Rust 侧做可打印性判断；字节本身原样交给 printf。）
        #[cfg(feature = "no-std")]
        {
            let bytes = unsafe { CStr::from_ptr(ptr) }.to_bytes();
            if core::str::from_utf8(bytes).is_ok() {
                unsafe { pf_cstr(ptr as *const u8) };
            } else {
                unsafe { pf_cstr(b"[invalid utf-8]\0".as_ptr()) };
            }
        }
        #[cfg(not(feature = "no-std"))]
        {
            let s = unsafe { CStr::from_ptr(ptr) };
            match s.to_str() {
                Ok(s) => {
                    if is_err {
                        eprint!("{s}");
                    } else {
                        print!("{s}");
                    }
                }
                Err(_) => {
                    if is_err {
                        eprint!("[invalid utf-8]");
                    } else {
                        print!("[invalid utf-8]");
                    }
                }
            }
        }

        // NOTE: this project currently doesn't expose JS_FreeCString in bindings.
        // v1: we keep the original behavior (leak per call in worst case) until bindings are extended.
    }

    #[cfg(feature = "no-std")]
    unsafe {
        pf_cstr(b"\n\0".as_ptr());
    }
    #[cfg(not(feature = "no-std"))]
    if is_err {
        eprintln!();
    } else {
        println!();
    }
}

#[cfg(feature = "no-std")]
use alloc::boxed::Box;
#[cfg(not(feature = "no-std"))]
use std::boxed::Box;

pub struct DefaultConsoleSingleton {
    enabled: bool,
}

impl Default for DefaultConsoleSingleton {
    fn default() -> Self {
        Self { enabled: true }
    }
}

impl crate::impls::ConsoleSingleton for DefaultConsoleSingleton {
    fn log(
        &mut self,
        _env: &mut mquickjs_rs::Env<'_>,
        args: Vec<mquickjs_rs::handles::local::Local<'_, mquickjs_rs::handles::local::Value>>,
    ) {
        // Keep v1 behavior: format via QuickJS C API.
        let Some(h) = mquickjs_rs::context::ContextToken::current() else {
            return;
        };
        let args: Vec<mquickjs_rs::mquickjs_ffi::JSValue> =
            args.into_iter().map(|v| v.as_raw()).collect();
        print_js_values(h.ctx, &args, false);
    }

    fn error(
        &mut self,
        _env: &mut mquickjs_rs::Env<'_>,
        args: Vec<mquickjs_rs::handles::local::Local<'_, mquickjs_rs::handles::local::Value>>,
    ) {
        let Some(h) = mquickjs_rs::context::ContextToken::current() else {
            return;
        };
        let args: Vec<mquickjs_rs::mquickjs_ffi::JSValue> =
            args.into_iter().map(|v| v.as_raw()).collect();
        print_js_values(h.ctx, &args, true);
    }

    fn enabled(&self) -> bool {
        self.enabled
    }
}

pub fn create_console_singleton() -> Box<dyn crate::api::ConsoleSingleton> {
    Box::new(DefaultConsoleSingleton::default())
}
