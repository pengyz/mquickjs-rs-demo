use crate::api::MyServiceSingleton;

pub struct DefaultMyService {
    version: String,
}

impl DefaultMyService {
    pub fn new() -> Self {
        Self {
            version: "1.0.0".to_string(),
        }
    }
}

impl MyServiceSingleton for DefaultMyService {
    fn hello(&mut self, name: String) -> String {
        format!("Hello, {}!", name)
    }

    fn version(&self) -> String {
        self.version.clone()
    }
}

pub fn create_my_service_singleton() -> Box<dyn MyServiceSingleton> {
    Box::new(DefaultMyService::new())
}