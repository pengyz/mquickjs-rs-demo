mquickjs_rs::ridl_include_module!();

pub mod impls {
    use crate::api::MFooClass;

    pub fn mping() -> i32 {
        7
    }

    pub fn m_foo_constructor() -> Box<dyn MFooClass> {
        Box::new(crate::impl_types::MFooImpl::default())
    }
}

mod impl_types;
