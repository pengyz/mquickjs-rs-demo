use crate::Context;
use crate::value::ValueRef;

/// Object封装
pub struct Object<'ctx> {
    pub value: ValueRef<'ctx>,
}

impl<'ctx> Object<'ctx> {
    pub fn get_property(&self, ctx: &'ctx Context, name: &str) -> Result<ValueRef<'ctx>, String> {
        self.value.get_property(ctx, name)
    }

    pub fn set_property(
        &self,
        ctx: &'ctx Context,
        name: &str,
        value: ValueRef<'ctx>,
    ) -> Result<(), String> {
        self.value.set_property(ctx, name, value)
    }
}
