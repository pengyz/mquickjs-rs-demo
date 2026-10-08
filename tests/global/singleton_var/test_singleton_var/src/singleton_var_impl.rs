use crate::api::TestSingletonVarSingleton;

pub struct DefaultTestSingletonVarSingleton;

impl TestSingletonVarSingleton for DefaultTestSingletonVarSingleton {
    // JS-only `var` 字段（Phase C）只挂在 JS 对象上，Rust impl 读不到。
    // describe() 返回固定串，供 JS 侧在直读字段后验证方法分派正常。
    fn describe(&mut self) -> String {
        "singleton-var-fields-ok".to_string()
    }
}

pub fn create_test_singleton_var_singleton() -> Box<dyn TestSingletonVarSingleton> {
    Box::new(DefaultTestSingletonVarSingleton)
}
