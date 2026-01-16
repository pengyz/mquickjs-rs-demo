use crate::api::TestImportUsingSingleton;

pub struct DefaultTestImportUsingSingleton;

impl TestImportUsingSingleton for DefaultTestImportUsingSingleton {
    fn ping(&mut self) -> i32 {
        0
    }
}

pub fn create_test_import_using_singleton() -> Box<dyn TestImportUsingSingleton> {
    Box::new(DefaultTestImportUsingSingleton)
}
