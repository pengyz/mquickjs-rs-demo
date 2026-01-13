use crate::{mquickjs_ffi, Context, ValueRef};

use std::ptr::NonNull;

pub type ClassId = i32;

pub struct Opaque<T>(NonNull<T>);

impl<T> Opaque<T> {
    pub unsafe fn new(ptr: *mut T) -> Option<Self> {
        NonNull::new(ptr).map(Self)
    }

    pub fn as_ptr(self) -> *mut T {
        self.0.as_ptr()
    }
}

pub struct ClassObject<'ctx> {
    value: ValueRef<'ctx>,
}

impl<'ctx> ClassObject<'ctx> {
    pub fn new(value: ValueRef<'ctx>) -> Self {
        Self { value }
    }

    pub fn into_value(self) -> ValueRef<'ctx> {
        self.value
    }

    pub fn class_id(&self, ctx: &'ctx Context) -> ClassId {
        unsafe { mquickjs_ffi::JS_GetClassID(ctx.ctx, self.value.as_raw()) as ClassId }
    }

    pub fn set_opaque<T>(&self, ctx: &'ctx Context, opaque: *mut T) {
        unsafe {
            mquickjs_ffi::JS_SetOpaque(ctx.ctx, self.value.as_raw(), opaque as *mut _);
        }
    }

    pub fn get_opaque<T>(&self, ctx: &'ctx Context) -> *mut T {
        unsafe { mquickjs_ffi::JS_GetOpaque(ctx.ctx, self.value.as_raw()) as *mut T }
    }

    pub fn get_opaque_nn<T>(&self, ctx: &'ctx Context) -> Option<Opaque<T>> {
        unsafe { Opaque::new(self.get_opaque::<T>(ctx)) }
    }
}

impl Context {
    pub fn new_class_object<'ctx>(&'ctx self, class_id: ClassId) -> ValueRef<'ctx> {
        let v = unsafe { mquickjs_ffi::JS_NewObjectClassUser(self.ctx, class_id) };
        ValueRef::new(v)
    }
}
