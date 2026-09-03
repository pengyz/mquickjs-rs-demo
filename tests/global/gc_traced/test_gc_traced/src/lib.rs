mquickjs_rs::ridl_include_module!();

pub mod impls {
    pub use crate::api::GcTracedNodeClass;

    pub fn gc_traced_node_constructor() -> Box<dyn GcTracedNodeClass> {
        Box::new(crate::node_impl::DefaultGcTracedNode::new())
    }
}

mod node_impl;
