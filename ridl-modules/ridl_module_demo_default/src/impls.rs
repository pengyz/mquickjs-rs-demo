
// Re-export generated singleton traits so glue can use stable paths.
pub use crate::api::DemoSingleton;

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

pub fn ridl_create_demo_singleton() -> Box<dyn crate::api::DemoSingleton> {
    create_demo_singleton()
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

// --- Class demo: Counter ---

pub struct Counter {
    value: i32,
}

impl crate::api::CounterClass for Counter {
    fn inc(&mut self, delta: i32) -> i32 {
        self.value += delta;
        self.value
    }

    fn get_value(&mut self) -> i32 {
        self.value
    }

    fn set_value(&mut self, v: i32) {
        self.value = v;
    }
}

pub fn counter_constructor() -> Box<dyn crate::api::CounterClass> {
    Box::new(Counter { value: 0 })
}

// Per-context proto state (erased)
#[repr(C)]
struct CounterProto {
    token: std::ffi::CString,
}

#[unsafe(no_mangle)]
pub extern "C" fn ridl_create_proto_counter() -> *mut crate::api::CounterProtoState {
    let st = CounterProto {
        token: std::ffi::CString::new("").unwrap(),
    };
    Box::into_raw(Box::new(st)) as *mut crate::api::CounterProtoState
}

#[unsafe(no_mangle)]
pub extern "C" fn ridl_drop_proto_counter(p: *mut crate::api::CounterProtoState) {
    if !p.is_null() {
        unsafe {
            drop(Box::from_raw(p as *mut CounterProto));
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn ridl_proto_get_counter_token(
    proto: *mut crate::api::CounterProtoState,
) -> *const core::ffi::c_char {
    if proto.is_null() {
        return core::ptr::null();
    }
    let st = unsafe { &mut *(proto as *mut CounterProto) };
    st.token.as_ptr()
}
