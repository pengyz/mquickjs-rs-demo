use crate::env::Env;
use crate::handles::any::Any;
use crate::handles::local::{Array, Local, Object, Value};
use crate::handles::scope::Scope;
use crate::mquickjs_ffi;

impl<'ctx> Local<'ctx, Value> {
    pub fn is_array(&self, scope: &Scope<'ctx>) -> bool {
        // mquickjs does not export JS_IsArray; use Array.isArray(x).
        // We implement it via: globalThis.Array.isArray(value)
        let ctx = scope.ctx_raw();
        unsafe {
            let global = mquickjs_ffi::JS_GetGlobalObject(ctx);
            if mquickjs_ffi::js_value_special_tag(global) == (mquickjs_ffi::JS_TAG_EXCEPTION as u32)
            {
                return false;
            }

            let array_ctor =
                mquickjs_ffi::JS_GetPropertyStr(ctx, global, b"Array\0".as_ptr() as *const _);
            if mquickjs_ffi::js_value_special_tag(array_ctor)
                == (mquickjs_ffi::JS_TAG_EXCEPTION as u32)
            {
                return false;
            }

            let is_array =
                mquickjs_ffi::JS_GetPropertyStr(ctx, array_ctor, b"isArray\0".as_ptr() as *const _);
            if mquickjs_ffi::js_value_special_tag(is_array)
                == (mquickjs_ffi::JS_TAG_EXCEPTION as u32)
            {
                return false;
            }

            // call: Array.isArray(value)
            mquickjs_ffi::JS_PushArg(ctx, self.as_raw());
            mquickjs_ffi::JS_PushArg(ctx, is_array);
            mquickjs_ffi::JS_PushArg(ctx, array_ctor);
            let ret = mquickjs_ffi::JS_Call(ctx, 1);
            if mquickjs_ffi::js_value_special_tag(ret) == (mquickjs_ffi::JS_TAG_EXCEPTION as u32) {
                return false;
            }

            let mut out = 0i32;
            if mquickjs_ffi::JS_ToInt32(ctx, &mut out, ret) != 0 {
                return false;
            }
            out != 0
        }
    }

    pub fn try_into_array(self, scope: &Scope<'ctx>) -> Result<Local<'ctx, Array>, String> {
        if self.is_array(scope) {
            Ok(Local::from_raw_for_same_ctx(self.as_raw()).with_ctx_id(self.ctx_id()))
        } else {
            Err("Value is not an array".to_string())
        }
    }
}

impl<'ctx> Local<'ctx, Array> {
    fn as_object(&self) -> Local<'ctx, Object> {
        Local::from_raw_for_same_ctx(self.as_raw()).with_ctx_id(self.ctx_id())
    }

    pub fn len(&self, env: &Env<'ctx>) -> Result<u32, String> {
        // length is on array prototype and is a getter.
        let length = self.as_object().get_property(env.scope(), "length")?;
        env.get_number(length).map(|n| n as u32)
    }

    pub fn get<'hs>(&self, env: &'hs mut Env<'ctx>, index: u32) -> Result<Any<'hs, 'ctx>, String> {
        let raw = unsafe {
            mquickjs_ffi::JS_GetPropertyUint32(env.scope().ctx_raw(), self.as_raw(), index)
        };
        if mquickjs_ffi::js_value_special_tag(raw) == (mquickjs_ffi::JS_TAG_EXCEPTION as u32) {
            return Err("Exception during array get".to_string());
        }
        Ok(Any::from_value(env.handle(env.scope().value(raw))))
    }

    pub fn set(
        &self,
        env: &Env<'ctx>,
        index: u32,
        value: Local<'ctx, Value>,
    ) -> Result<(), String> {
        let r = unsafe {
            mquickjs_ffi::JS_SetPropertyUint32(
                env.scope().ctx_raw(),
                self.as_raw(),
                index,
                value.as_raw(),
            )
        };
        if mquickjs_ffi::js_value_special_tag(r) == (mquickjs_ffi::JS_TAG_EXCEPTION as u32) {
            return Err("Exception during array set".to_string());
        }
        Ok(())
    }

    pub fn push(&self, env: &Env<'ctx>, value: Local<'ctx, Value>) -> Result<u32, String> {
        let index = self.len(env)?;
        self.set(env, index, value)?;
        Ok(index + 1)
    }

    pub fn pop<'hs>(&self, env: &'hs mut Env<'ctx>) -> Result<Any<'hs, 'ctx>, String> {
        let len = self.len(&*env)?;
        if len == 0 {
            return Ok(Any::from_value(
                env.handle(env.scope().value(mquickjs_ffi::JS_UNDEFINED)),
            ));
        }

        let ctx = env.scope().ctx_raw();
        let last_index = len - 1;

        // Fetch value first (needs &mut Env to create a rooted Any).
        let v = self.get(env, last_index)?;

        // Shrink by setting `length`.
        let new_len = unsafe { mquickjs_ffi::JS_NewUint32(ctx, last_index) };
        if mquickjs_ffi::js_value_special_tag(new_len) == (mquickjs_ffi::JS_TAG_EXCEPTION as u32) {
            return Err("Exception during JS_NewUint32".to_string());
        }

        let r = unsafe {
            mquickjs_ffi::JS_SetPropertyStr(
                ctx,
                self.as_raw(),
                b"length\0".as_ptr() as *const _,
                new_len,
            )
        };
        if mquickjs_ffi::js_value_special_tag(r) == (mquickjs_ffi::JS_TAG_EXCEPTION as u32) {
            return Err("Exception during array pop (set length)".to_string());
        }

        Ok(v)
    }
}
