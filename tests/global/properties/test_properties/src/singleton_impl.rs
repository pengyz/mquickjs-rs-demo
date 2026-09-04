use crate::api::PropertyTestSingletonSingleton;

pub struct DefaultPropertyTestSingleton {
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

impl DefaultPropertyTestSingleton {
    pub fn new() -> Self {
        Self {
            bool_prop: true,
            i32_prop: 42,
            i64_prop: 1234567890123,
            f32_prop: 3.14,
            f64_prop: 2.718281828459045,
            string_prop: "hello".to_string(),
            mutable_bool: false,
            mutable_i32: 0,
            mutable_string: String::new(),
        }
    }
}

impl PropertyTestSingletonSingleton for DefaultPropertyTestSingleton {
    fn bool_prop(&self) -> bool {
        self.bool_prop
    }

    fn i32_prop(&self) -> i32 {
        self.i32_prop
    }

    fn i64_prop(&self) -> i64 {
        self.i64_prop
    }

    fn f32_prop(&self) -> f32 {
        self.f32_prop
    }

    fn f64_prop(&self) -> f64 {
        self.f64_prop
    }

    fn string_prop(&self) -> String {
        self.string_prop.clone()
    }

    fn mutable_bool(&self) -> bool {
        self.mutable_bool
    }

    fn set_mutable_bool(&mut self, value: bool) {
        self.mutable_bool = value;
    }

    fn mutable_i32(&self) -> i32 {
        self.mutable_i32
    }

    fn set_mutable_i32(&mut self, value: i32) {
        self.mutable_i32 = value;
    }

    fn mutable_string(&self) -> String {
        self.mutable_string.clone()
    }

    fn set_mutable_string(&mut self, value: String) {
        self.mutable_string = value;
    }
}

pub fn create_property_test_singleton_singleton() -> Box<dyn PropertyTestSingletonSingleton> {
    Box::new(DefaultPropertyTestSingleton::new())
}