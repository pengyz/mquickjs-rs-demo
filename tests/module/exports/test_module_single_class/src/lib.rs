mquickjs_rs::ridl_include_module!();

pub mod impls {
    use crate::api::OnlyClass;

    pub fn only_constructor() -> Box<dyn OnlyClass> {
        Box::new(crate::impl_types::OnlyImpl::default())
    }
}

mod impl_types;
