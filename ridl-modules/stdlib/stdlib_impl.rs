//! RIDL stdlib: Rust-side implementations.
//!
//! NOTE: This file intentionally starts minimal.
//! We are currently focusing on getting `console.log(content: string)` working
//! end-to-end via the build-time stdlib injection mechanism.

use std::ffi::CStr;

use mquickjs_rs::mquickjs_ffi::{JSContext, JSValue};

use crate::generated::impls::ConsoleSingleton;

#[repr(C)]
pub struct RidlConsoleVTable {
    pub self_ptr: *mut std::ffi::c_void,
    pub drop_fn: unsafe extern "C" fn(*mut std::ffi::c_void),
    pub log_fn: unsafe extern "C" fn(
        *mut std::ffi::c_void,
        *mut JSContext,
        mquickjs_rs::mquickjs_ffi::JSValue,
        std::os::raw::c_int,
        *const mquickjs_rs::mquickjs_ffi::JSValue,
    ),
    pub error_fn: unsafe extern "C" fn(
        *mut std::ffi::c_void,
        *mut JSContext,
        mquickjs_rs::mquickjs_ffi::JSValue,
        std::os::raw::c_int,
        *const mquickjs_rs::mquickjs_ffi::JSValue,
    ),
    pub enabled_fn: unsafe extern "C" fn(*mut std::ffi::c_void) -> std::os::raw::c_int,
}

unsafe extern "C" fn console_drop(self_ptr: *mut std::ffi::c_void) {
    if self_ptr.is_null() {
        return;
    }
    let _ = Box::from_raw(self_ptr as *mut DefaultConsoleSingleton);
}

unsafe extern "C" fn console_log(
    self_ptr: *mut std::ffi::c_void,
    ctx: *mut JSContext,
    _this_val: mquickjs_rs::mquickjs_ffi::JSValue,
    argc: std::os::raw::c_int,
    argv: *const mquickjs_rs::mquickjs_ffi::JSValue,
) {
    if self_ptr.is_null() {
        return;
    }
    let s = &mut *(self_ptr as *mut DefaultConsoleSingleton);
    let args = if argv.is_null() || argc <= 0 {
        Vec::new()
    } else {
        core::slice::from_raw_parts(argv, argc as usize).to_vec()
    };
    s.log(ctx, args);
}

unsafe extern "C" fn console_error(
    self_ptr: *mut std::ffi::c_void,
    ctx: *mut JSContext,
    _this_val: mquickjs_rs::mquickjs_ffi::JSValue,
    argc: std::os::raw::c_int,
    argv: *const mquickjs_rs::mquickjs_ffi::JSValue,
) {
    if self_ptr.is_null() {
        return;
    }
    let s = &mut *(self_ptr as *mut DefaultConsoleSingleton);
    let args = if argv.is_null() || argc <= 0 {
        Vec::new()
    } else {
        core::slice::from_raw_parts(argv, argc as usize).to_vec()
    };
    s.error(ctx, args);
}

unsafe extern "C" fn console_enabled(self_ptr: *mut std::ffi::c_void) -> std::os::raw::c_int {
    if self_ptr.is_null() {
        return 0;
    }
    let s = &*(self_ptr as *mut DefaultConsoleSingleton);
    if s.enabled() { 1 } else { 0 }
}

pub fn ridl_console_vtable_create() -> RidlConsoleVTable {
    let self_ptr = Box::into_raw(Box::new(DefaultConsoleSingleton::default())) as *mut std::ffi::c_void;
    RidlConsoleVTable {
        self_ptr,
        drop_fn: console_drop,
        log_fn: console_log,
        error_fn: console_error,
        enabled_fn: console_enabled,
    }
}

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
    fn log(&mut self, ctx: *mut JSContext, args: Vec<JSValue>) {
        print_js_values(ctx, &args, false);
        println!();
    }

    fn error(&mut self, ctx: *mut JSContext, args: Vec<JSValue>) {
        print_js_values(ctx, &args, true);
        eprintln!();
    }

    fn enabled(&self) -> bool {
        self.enabled
    }
}

pub fn create_console_singleton() -> Box<dyn crate::impls::ConsoleSingleton> {
    Box::new(DefaultConsoleSingleton::default())
}

pub fn rust_console_log(
    ctx: *mut std::ffi::c_void,
    _this_val: mquickjs_rs::mquickjs_ffi::JSValue,
    args: Vec<JSValue>,
) -> mquickjs_rs::mquickjs_ffi::JSValue {
    let ctx = ctx as *mut JSContext;
    let mut h = match unsafe { mquickjs_rs::context::ContextHandle::from_js_ctx(ctx) } {
        Some(h) => h,
        None => {
            return unsafe {
                mquickjs_rs::mquickjs_ffi::JS_ThrowError(
                    ctx,
                    mquickjs_rs::mquickjs_ffi::JSObjectClassEnum_JS_CLASS_TYPE_ERROR,
                    c"Context not initialized".as_ptr(),
                )
            };
        }
    };

    let ext_ptr = h.inner.ridl_ext_ptr();
    if ext_ptr.is_null() {
        return unsafe {
            mquickjs_rs::mquickjs_ffi::JS_ThrowError(
                ctx,
                mquickjs_rs::mquickjs_ffi::JSObjectClassEnum_JS_CLASS_TYPE_ERROR,
                c"RIDL context not initialized".as_ptr(),
            )
        };
    }

    let Some(slot_ptr) = (unsafe { ::mquickjs_rs::ridl_ext_access::ridl_get_erased_singleton_slot(ext_ptr, 0) }) else {
        return unsafe {
            mquickjs_rs::mquickjs_ffi::JS_ThrowError(
                ctx,
                mquickjs_rs::mquickjs_ffi::JSObjectClassEnum_JS_CLASS_TYPE_ERROR,
                c"RIDL ctx_ext vtable not initialized".as_ptr(),
            )
        };
    };
    let slot = unsafe { &mut *slot_ptr };

    if !slot.is_set() {
        return unsafe {
            mquickjs_rs::mquickjs_ffi::JS_ThrowError(
                ctx,
                mquickjs_rs::mquickjs_ffi::JSObjectClassEnum_JS_CLASS_TYPE_ERROR,
                c"console singleton not initialized".as_ptr(),
            )
        };
    }

    let holder_ptr = slot.ptr() as *mut Box<dyn crate::impls::ConsoleSingleton>;
    let s: &mut dyn crate::impls::ConsoleSingleton = unsafe { &mut **holder_ptr };
    s.log(ctx, args);
    mquickjs_rs::mquickjs_ffi::JS_UNDEFINED
}

pub fn rust_console_error(
    ctx: *mut std::ffi::c_void,
    _this_val: mquickjs_rs::mquickjs_ffi::JSValue,
    args: Vec<JSValue>,
) -> mquickjs_rs::mquickjs_ffi::JSValue {
    let ctx = ctx as *mut JSContext;
    let mut h = match unsafe { mquickjs_rs::context::ContextHandle::from_js_ctx(ctx) } {
        Some(h) => h,
        None => {
            return unsafe {
                mquickjs_rs::mquickjs_ffi::JS_ThrowError(
                    ctx,
                    mquickjs_rs::mquickjs_ffi::JSObjectClassEnum_JS_CLASS_TYPE_ERROR,
                    c"Context not initialized".as_ptr(),
                )
            };
        }
    };

    let ext_ptr = h.inner.ridl_ext_ptr();
    if ext_ptr.is_null() {
        return unsafe {
            mquickjs_rs::mquickjs_ffi::JS_ThrowError(
                ctx,
                mquickjs_rs::mquickjs_ffi::JSObjectClassEnum_JS_CLASS_TYPE_ERROR,
                c"RIDL context not initialized".as_ptr(),
            )
        };
    }

    let Some(slot_ptr) = (unsafe { ::mquickjs_rs::ridl_ext_access::ridl_get_erased_singleton_slot(ext_ptr, 0) }) else {
        return unsafe {
            mquickjs_rs::mquickjs_ffi::JS_ThrowError(
                ctx,
                mquickjs_rs::mquickjs_ffi::JSObjectClassEnum_JS_CLASS_TYPE_ERROR,
                c"RIDL ctx_ext vtable not initialized".as_ptr(),
            )
        };
    };
    let slot = unsafe { &mut *slot_ptr };

    if !slot.is_set() {
        return unsafe {
            mquickjs_rs::mquickjs_ffi::JS_ThrowError(
                ctx,
                mquickjs_rs::mquickjs_ffi::JSObjectClassEnum_JS_CLASS_TYPE_ERROR,
                c"console singleton not initialized".as_ptr(),
            )
        };
    }

    let holder_ptr = slot.ptr() as *mut Box<dyn crate::impls::ConsoleSingleton>;
    let s: &mut dyn crate::impls::ConsoleSingleton = unsafe { &mut **holder_ptr };
    s.error(ctx, args);
    mquickjs_rs::mquickjs_ffi::JS_UNDEFINED
}

pub fn rust_console_get_enabled(ctx: *mut std::ffi::c_void) -> bool {
    let ctx = ctx as *mut JSContext;
    let h = match unsafe { mquickjs_rs::context::ContextHandle::from_js_ctx(ctx) } {
        Some(h) => h,
        None => return false,
    };

    let ext_ptr = h.inner.ridl_ext_ptr();
    if ext_ptr.is_null() {
        return false;
    }

    let Some(slot_ptr) = (unsafe { ::mquickjs_rs::ridl_ext_access::ridl_get_erased_singleton_slot(ext_ptr, 0) }) else {
        return false;
    };
    let slot = unsafe { &mut *slot_ptr };
    if !slot.is_set() {
        return false;
    }

    let holder_ptr = slot.ptr() as *mut Box<dyn crate::impls::ConsoleSingleton>;
    let s: &mut dyn crate::impls::ConsoleSingleton = unsafe { &mut **holder_ptr };
    s.enabled()
}
