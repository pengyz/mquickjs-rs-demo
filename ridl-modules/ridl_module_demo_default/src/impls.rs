
// Re-export generated singleton traits so glue can use stable paths.
pub use crate::generated::api::DemoSingleton;

struct DefaultDemoSingleton;

impl DemoSingleton for DefaultDemoSingleton {
    fn ping(
        &mut self,
        _ctx: *mut mquickjs_rs::mquickjs_ffi::JSContext,
        _args: Vec<mquickjs_rs::mquickjs_ffi::JSValue>,
    ) {
        // Default demo singleton: no-op.
        // The JS smoke test only validates the singleton wiring path.
    }
}

pub fn create_demo_singleton() -> Box<dyn DemoSingleton> {
    Box::new(DefaultDemoSingleton)
}

// --- Default-mode function demos ---

pub fn default_echo_str(s: *const std::os::raw::c_char) -> String {
    if s.is_null() {
        return "".to_string();
    }
    unsafe { std::ffi::CStr::from_ptr(s).to_string_lossy().into_owned() }
}

pub fn default_add_i32(a: i32, b: i32) -> i32 {
    a + b
}

pub fn default_not_bool(v: bool) -> bool {
    !v
}

pub fn default_add_f64(a: f64, b: f64) -> f64 {
    a + b
}

pub fn default_id_any(v: mquickjs_rs::mquickjs_ffi::JSValue) -> mquickjs_rs::mquickjs_ffi::JSValue {
    v
}

pub fn default_void_ok() {}
