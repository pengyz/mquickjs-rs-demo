use crate::api::{TestClassSingleton, UserClass};

pub struct DefaultTestClassSingleton;

impl TestClassSingleton for DefaultTestClassSingleton {
    fn make_user(&mut self, name: String) -> Box<dyn crate::api::UserClass> {
        Box::new(DefaultUser { name })
    }
}

pub struct DefaultUser {
    pub name: String,
}

impl UserClass for DefaultUser {
    fn get_name(&mut self) -> String {
        self.name.clone()
    }
}

