use crate::api::PropertyTestClassClass;

pub struct DefaultPropertyTestClass {
    bool_prop: bool,
    i32_prop: i32,
    i64_prop: i64,
    f32_prop: f32,
    f64_prop: f64,
    string_prop: String,
    mutable_bool: bool,
    mutable_i32: i32,
    mutable_string: String,
}

impl DefaultPropertyTestClass {
    pub fn new() -> Self {
        Self {
            bool_prop: false,
            i32_prop: 100,
            i64_prop: 9876543210,
            f32_prop: 1.23,
            f64_prop: 4.56789,
            string_prop: "world".to_string(),
            mutable_bool: true,
            mutable_i32: 50,
            mutable_string: "initial".to_string(),
        }
    }
}

impl PropertyTestClassClass for DefaultPropertyTestClass {
    fn get_bool_prop(&mut self) -> bool {
        self.bool_prop
    }

    fn get_i32_prop(&mut self) -> i32 {
        self.i32_prop
    }

    fn get_i64_prop(&mut self) -> i64 {
        self.i64_prop
    }

    fn get_f32_prop(&mut self) -> f32 {
        self.f32_prop
    }

    fn get_f64_prop(&mut self) -> f64 {
        self.f64_prop
    }

    fn get_string_prop(&mut self) -> String {
        self.string_prop.clone()
    }

    fn get_mutable_bool(&mut self) -> bool {
        self.mutable_bool
    }

    fn set_mutable_bool(&mut self, value: bool) {
        self.mutable_bool = value;
    }

    fn get_mutable_i32(&mut self) -> i32 {
        self.mutable_i32
    }

    fn set_mutable_i32(&mut self, value: i32) {
        self.mutable_i32 = value;
    }

    fn get_mutable_string(&mut self) -> String {
        self.mutable_string.clone()
    }

    fn set_mutable_string(&mut self, value: String) {
        self.mutable_string = value;
    }

    fn gc_mark(&self, _mf: *const mquickjs_rs::mquickjs_ffi::JSMarkFunc) {
        // No GC-traced fields
    }
}

pub fn create_property_test_class_class() -> Box<dyn PropertyTestClassClass> {
    Box::new(DefaultPropertyTestClass::new())
}

pub fn property_test_class_constructor() -> Box<dyn PropertyTestClassClass> {
    Box::new(DefaultPropertyTestClass::new())
}