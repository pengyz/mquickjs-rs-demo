pub struct DefaultTestGcSingleton;

impl TestGcSingleton for DefaultTestGcSingleton {
    fn make_node(&mut self) -> Box<dyn crate::api::NodeClass> {
        Box::new(super::node_impl::DefaultNode::new())
    }
}
