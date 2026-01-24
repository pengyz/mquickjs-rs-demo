use crate::handles::local::{Local, Object, Value};
use crate::handles::scope::Scope;
use crate::mquickjs_ffi;

impl<'ctx> Local<'ctx, Value> {
    pub fn is_object(&self, scope: &Scope<'ctx>) -> bool {
        // mquickjs exposes JS_IsObject in C but it is not bound in mquickjs-sys yet.
        // Use a conservative approximation: exclude common primitives.
        if (unsafe { mquickjs_ffi::JS_IsNumber(scope.ctx(), self.as_raw()) }) != 0 {
            return false;
        }
        if (unsafe { mquickjs_ffi::JS_IsString(scope.ctx(), self.as_raw()) }) != 0 {
            return false;
        }
        // booleans/null/undefined/exception are tagged as special values.
        let special =
            (self.as_raw() as u32) & ((1u32 << (mquickjs_ffi::JS_TAG_SPECIAL_BITS as u32)) - 1);
        if special == mquickjs_ffi::JS_TAG_BOOL as u32
            || special == mquickjs_ffi::JS_TAG_NULL as u32
            || special == mquickjs_ffi::JS_TAG_UNDEFINED as u32
            || special == mquickjs_ffi::JS_TAG_EXCEPTION as u32
        {
            return false;
        }
        true
    }

    pub fn try_into_object(self, scope: &Scope<'ctx>) -> Result<Local<'ctx, Object>, String> {
        if self.is_object(scope) {
            Ok(Local::from_raw_for_same_ctx(self.as_raw()).with_ctx_id(self.ctx_id()))
        } else {
            Err("Value is not an object".to_string())
        }
    }
}

impl<'ctx> Local<'ctx, Object> {
    pub fn get_property(
        &self,
        scope: &Scope<'ctx>,
        name: &str,
    ) -> Result<Local<'ctx, Value>, String> {
        let c_name =
            std::ffi::CString::new(name).map_err(|_| "Invalid property name".to_string())?;
        let raw =
            unsafe { mquickjs_ffi::JS_GetPropertyStr(scope.ctx(), self.as_raw(), c_name.as_ptr()) };
        if (raw as u32) & ((1u32 << (mquickjs_ffi::JS_TAG_SPECIAL_BITS as u32)) - 1)
            == (mquickjs_ffi::JS_TAG_EXCEPTION as u32)
        {
            return Err("Exception during get_property".to_string());
        }
        Ok(scope.value(raw))
    }

    pub fn set_property(
        &self,
        scope: &Scope<'ctx>,
        name: &str,
        value: Local<'ctx, Value>,
    ) -> Result<(), String> {
        let c_name =
            std::ffi::CString::new(name).map_err(|_| "Invalid property name".to_string())?;
        // QuickJS property setters consume the value in many APIs; in our engine model,
        // the GC will keep the value alive when it becomes reachable.
        let r = unsafe {
            mquickjs_ffi::JS_SetPropertyStr(
                scope.ctx(),
                self.as_raw(),
                c_name.as_ptr(),
                value.as_raw(),
            )
        };
        if r == 0 {
            return Err("Exception during set_property".to_string());
        }
        Ok(())
    }
}
