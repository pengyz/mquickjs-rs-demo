use crate::handles::handle::Handle;
use crate::handles::handle_scope::HandleScope;
use crate::handles::local::{Array, Local, Object, Value};
use crate::handles::scope::Scope;
use crate::mquickjs_ffi;

pub struct Env<'ctx> {
    scope: &'ctx Scope<'ctx>,
    hs: HandleScope<'ctx>,
}

impl<'ctx> Env<'ctx> {
    pub fn new(scope: &'ctx Scope<'ctx>) -> Self {
        Self {
            scope,
            hs: HandleScope::new(scope),
        }
    }

    pub fn scope(&self) -> &'ctx Scope<'ctx> {
        self.scope
    }

    pub fn handle_scope(&mut self) -> &mut HandleScope<'ctx> {
        &mut self.hs
    }

    pub fn handle<'hs, T>(&'hs mut self, v: Local<'ctx, T>) -> Handle<'hs, 'ctx, T> {
        self.hs.handle(v)
    }

    pub fn obj<'hs>(&'hs mut self) -> Result<Handle<'hs, 'ctx, Object>, String> {
        let raw = unsafe { mquickjs_ffi::JS_NewObject(self.scope.ctx_raw()) };
        if mquickjs_ffi::js_value_special_tag(raw) == (mquickjs_ffi::JS_TAG_EXCEPTION as u32) {
            return Err("Exception during JS_NewObject".to_string());
        }
        Ok(self.handle(self.scope.value(raw).try_into_object(self.scope)?))
    }

    pub fn array<'hs>(&'hs mut self) -> Result<Handle<'hs, 'ctx, Array>, String> {
        self.array_with_len(0)
    }

    pub fn array_with_len<'hs>(&'hs mut self, len: u32) -> Result<Handle<'hs, 'ctx, Array>, String> {
        let raw = unsafe { mquickjs_ffi::JS_NewArray(self.scope.ctx_raw(), len as i32) };
        if mquickjs_ffi::js_value_special_tag(raw) == (mquickjs_ffi::JS_TAG_EXCEPTION as u32) {
            return Err("Exception during JS_NewArray".to_string());
        }
        Ok(self.handle(self.scope.value(raw).try_into_array(self.scope)?))
    }

    pub fn str<'hs>(&'hs mut self, s: &str) -> Result<Handle<'hs, 'ctx, Value>, String> {
        let c = std::ffi::CString::new(s).map_err(|_| "Invalid string".to_string())?;
        let raw = unsafe { mquickjs_ffi::JS_NewString(self.scope.ctx_raw(), c.as_ptr()) };
        if mquickjs_ffi::js_value_special_tag(raw) == (mquickjs_ffi::JS_TAG_EXCEPTION as u32) {
            return Err("Exception during JS_NewString".to_string());
        }
        Ok(self.handle(self.scope.value(raw)))
    }

    pub fn get_number(&self, v: Local<'ctx, Value>) -> Result<f64, String> {
        let mut result = 0.0;
        let ret = unsafe { mquickjs_ffi::JS_ToNumber(self.scope.ctx_raw(), &mut result, v.as_raw()) };
        if ret != 0 {
            return Err("Failed to convert Value to number".to_string());
        }
        Ok(result)
    }

    pub fn int_local(&self, v: i32) -> Result<Local<'ctx, Value>, String> {
        let raw = unsafe { mquickjs_ffi::JS_NewInt32(self.scope.ctx_raw(), v) };
        if mquickjs_ffi::js_value_special_tag(raw) == (mquickjs_ffi::JS_TAG_EXCEPTION as u32) {
            return Err("Exception during JS_NewInt32".to_string());
        }
        Ok(self.scope.value(raw))
    }

    pub fn uint_local(&self, v: u32) -> Result<Local<'ctx, Value>, String> {
        let raw = unsafe { mquickjs_ffi::JS_NewUint32(self.scope.ctx_raw(), v) };
        if mquickjs_ffi::js_value_special_tag(raw) == (mquickjs_ffi::JS_TAG_EXCEPTION as u32) {
            return Err("Exception during JS_NewUint32".to_string());
        }
        Ok(self.scope.value(raw))
    }

    pub fn get_string(&self, v: Local<'ctx, Value>) -> Result<String, String> {
        let mut cstr_buf = mquickjs_ffi::JSCStringBuf { buf: [0; 5] };
        let result_ptr = unsafe { mquickjs_ffi::JS_ToCString(self.scope.ctx_raw(), v.as_raw(), &mut cstr_buf) };
        if result_ptr.is_null() {
            return Err("Failed to convert Value to string".to_string());
        }
        Ok(unsafe { std::ffi::CStr::from_ptr(result_ptr) }
            .to_string_lossy()
            .into_owned())
    }
}
