use crate::value::Value;
use crate::Context;

/// Object封装
pub struct Object<'ctx> {
    pub value: Value<'ctx>,
}

impl<'ctx> Object<'ctx> {
    pub fn get_property(&self, ctx: &'ctx Context, name: &str) -> Result<Value<'ctx>, String> {
        self.value.get_property(ctx, name)
    }

    pub fn set_property(&self, ctx: &'ctx Context, name: &str, value: Value<'ctx>) -> Result<(), String> {
        self.value.set_property(ctx, name, value)
    }
}