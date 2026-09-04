use crate::api::ConstantTestClassClass;

pub struct DefaultConstantTestClass {
    counter: i32,
    status: String,
}

impl DefaultConstantTestClass {
    pub fn new() -> Self {
        Self {
            counter: 0,
            status: "idle".to_string(),
        }
    }
}

impl ConstantTestClassClass for DefaultConstantTestClass {
    fn get_max_size(&mut self) -> i32 {
        100
    }

    fn get_pi(&mut self) -> f64 {
        3.14159
    }

    fn get_name(&mut self) -> String {
        "test".to_string()
    }

    fn get_enabled(&mut self) -> bool {
        true
    }

    fn get_counter(&mut self) -> i32 {
        self.counter
    }

    fn set_counter(&mut self, value: i32) {
        self.counter = value;
    }

    fn get_status(&mut self) -> String {
        self.status.clone()
    }

    fn set_status(&mut self, value: String) {
        self.status = value;
    }

    fn gc_mark(&self, _mf: *const mquickjs_rs::mquickjs_ffi::JSMarkFunc) {
        // No GC-traced fields
    }
}

pub fn create_constant_test_class_class() -> Box<dyn ConstantTestClassClass> {
    Box::new(DefaultConstantTestClass::new())
}

pub fn constant_test_class_constructor() -> Box<dyn ConstantTestClassClass> {
    Box::new(DefaultConstantTestClass::new())
}