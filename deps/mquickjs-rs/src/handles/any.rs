use crate::env::Env;
use crate::handles::handle::Handle;
use crate::handles::local::{Local, Value};

pub struct Any<'hs, 'ctx>(Handle<'hs, 'ctx, Value>);

impl<'hs, 'ctx> Any<'hs, 'ctx> {
    pub fn from_value(v: Handle<'hs, 'ctx, Value>) -> Self {
        Self(v)
    }

    pub fn as_value(&self) -> &Handle<'hs, 'ctx, Value> {
        &self.0
    }

    pub fn as_raw(&self) -> crate::mquickjs_ffi::JSValue {
        self.0.as_raw()
    }

    pub fn as_local(&self, env: &Env<'ctx>) -> Local<'ctx, Value> {
        env.scope().value(self.as_raw())
    }
}
