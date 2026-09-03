use std::sync::atomic::{AtomicI32, Ordering};

use mquickjs_rs::handles::local::{Local, Value};

static FINALIZER_COUNT: AtomicI32 = AtomicI32::new(0);

pub struct DefaultNode {
    held: Option<mquickjs_rs::Root<Value>>,
    self_obj: Option<mquickjs_rs::Root<Value>>,
}

impl DefaultNode {
    pub fn new() -> Self {
        Self {
            held: None,
            self_obj: None,
        }
    }
}

impl Drop for DefaultNode {
    fn drop(&mut self) {
        FINALIZER_COUNT.fetch_add(1, Ordering::SeqCst);
    }
}

impl NodeClass for DefaultNode {
    fn set_held<'ctx>(&mut self, env: &mut mquickjs_rs::Env<'ctx>, v: mquickjs_rs::handles::object::Object<'ctx>) {
        let local = env.scope().value(v.as_raw());
        self.held = Some(mquickjs_rs::Root::new(env.scope(), local));
    }

    fn clear_held(&mut self) {
        self.held = None;
    }

    fn make_cycle<'ctx>(&mut self, env: &mut mquickjs_rs::Env<'ctx>) {
        // Capture receiver JS object into a Root.
        // NOTE: This relies on the glue calling the trait method with a validated receiver.
        // We fetch `this` by round-tripping through an exported helper on Env.
        let this = env.this();
        let this_local: Local<'ctx, Value> = env.scope().value(this.as_raw());
        self.self_obj = Some(mquickjs_rs::Root::new(env.scope(), this_local));

        if let (Some(ref held), Some(ref this_root)) = (&self.held, &self.self_obj) {
            unsafe {
                let name = std::ffi::CString::new("back").unwrap();
                let _ = mquickjs_rs::mquickjs_ffi::JS_SetPropertyStr(
                    env.scope().ctx_raw(),
                    held.as_raw(),
                    name.as_ptr(),
                    this_root.as_raw(),
                );
            }
        }
    }

    fn drop_all(&mut self) {
        self.held = None;
        self.self_obj = None;
    }

    fn finalizer_count(&mut self) -> i32 {
        FINALIZER_COUNT.load(Ordering::SeqCst)
    }

    fn root_held(&mut self) {
        // Already rooted via Root<T>.
    }

    fn unroot_held(&mut self) {
        self.held = None;
    }
}

