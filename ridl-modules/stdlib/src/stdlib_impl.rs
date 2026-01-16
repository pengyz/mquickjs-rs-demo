//! RIDL stdlib: Rust-side implementations.
//!
//! NOTE: This file intentionally starts minimal.
//! We are currently focusing on getting `console.log(content: string)` working
//! end-to-end via the build-time stdlib injection mechanism.

use std::ffi::CStr;

use mquickjs_rs::mquickjs_ffi::{JSContext, JSValue};


fn print_js_values(ctx: *mut JSContext, args: &[JSValue], is_err: bool) {
    for (i, v) in args.iter().copied().enumerate() {
        if i != 0 {
            if is_err {
                eprint!(" ");
            } else {
                print!(" ");
            }
        }

        let mut buf = mquickjs_rs::mquickjs_ffi::JSCStringBuf { buf: [0u8; 5] };
        let ptr = unsafe { mquickjs_rs::mquickjs_ffi::JS_ToCString(ctx, v, &mut buf as *mut _) };
        if ptr.is_null() {
            if is_err {
                eprint!("[toString failed]");
            } else {
                print!("[toString failed]");
            }
            continue;
        }

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

        // NOTE: this project currently doesn't expose JS_FreeCString in bindings.
        // v1: we keep the original behavior (leak per call in worst case) until bindings are extended.
    }
}

pub struct DefaultConsoleSingleton {
    enabled: bool,
}

impl Default for DefaultConsoleSingleton {
    fn default() -> Self {
        Self { enabled: true }
    }
}

impl crate::impls::ConsoleSingleton for DefaultConsoleSingleton {
    fn log(&mut self, args: Vec<mquickjs_rs::ValueRef<'_>>) {
        // Keep v1 behavior: format via QuickJS C API.
        // NOTE: ValueRef is a borrowed view; we can pass through raw values.
        // TODO: if we later want to avoid using C string conversion here, implement formatting in Rust.
        let Some(h) = mquickjs_rs::context::ContextHandle::current() else {
            return;
        };
        print_js_values(h.ctx, &args.iter().map(|v| v.as_raw()).collect::<Vec<_>>(), false);
        println!();
    }

    fn error(&mut self, args: Vec<mquickjs_rs::ValueRef<'_>>) {
        let Some(h) = mquickjs_rs::context::ContextHandle::current() else {
            return;
        };
        print_js_values(h.ctx, &args.iter().map(|v| v.as_raw()).collect::<Vec<_>>(), true);
        eprintln!();
    }

    fn enabled(&self) -> bool {
        self.enabled
    }
}

pub fn create_console_singleton() -> Box<dyn crate::api::ConsoleSingleton> {
    Box::new(DefaultConsoleSingleton::default())
}
