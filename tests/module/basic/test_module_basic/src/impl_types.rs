use crate::api::{MBarClass, MFooClass};

#[derive(Default)]
pub struct MFooImpl;

#[derive(Default)]
pub struct MBarImpl {
    pub v: i32,
}

impl MFooClass for MFooImpl {
    fn add(&mut self, a: i32, b: i32) -> i32 {
        a + b
    }

    fn echo_bool(&mut self, v: bool) -> bool {
        v
    }

    fn echo_i32(&mut self, v: i32) -> i32 {
        v
    }

    fn echo_f64(&mut self, v: f64) -> f64 {
        v
    }

    fn echo_f32(&mut self, v: f32) -> f32 {
        v
    }

    fn echo_i64(&mut self, v: i64) -> i64 {
        v
    }

    fn echo_string(&mut self, v: String) -> String {
        v
    }

    fn echo_any(
        &mut self,
        _env: &mut mquickjs_rs::Env<'_>,
        _v: mquickjs_rs::handles::local::Local<'_, mquickjs_rs::handles::local::Value>,
    ) {
    }

    fn make_bar(&mut self, v: i32) -> Box<dyn MBarClass> {
        Box::new(MBarImpl { v })
    }

    fn use_bar(&mut self, mut b: Box<dyn MBarClass>) -> i32 {
        b.get_v()
    }

    fn get_v(&mut self) -> i32 {
        777
    }
}

impl MBarClass for MBarImpl {
    fn get_v(&mut self) -> i32 {
        self.v
    }
}
