use mquickjs_rs::mquickjs_ffi;

pub struct Context {
    inner: mquickjs_rs::Context,
}

impl Context {
    pub fn new(memory_capacity: usize) -> Result<Self, Box<dyn std::error::Error>> {
        let inner = mquickjs_rs::Context::new(memory_capacity)?;

        // Context-level RIDL init is application-specific and only available when this crate
        // is built with ridl-extensions.
        #[cfg(feature = "ridl-extensions")]
        unsafe {
            crate::ridl_runtime_support::ridl_context_init(inner.ctx as *mut mquickjs_ffi::JSContext);
        }

        Ok(Self { inner })
    }

    pub fn default() -> Self {
        Self::new(1024 * 1024).expect("failed to create JSContext")
    }

    pub fn eval(&mut self, code: &str) -> Result<String, String> {
        self.inner.eval(code)
    }
}

impl Default for Context {
    fn default() -> Self {
        Self::new(1024 * 1024).expect("failed to create JSContext")
    }
}
