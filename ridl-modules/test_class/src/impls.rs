pub struct Basic {
    value: i32,
}

impl crate::api::BasicClass for Basic {
    fn add(&mut self, a: i32, b: i32) -> i32 {
        a + b
    }

    fn get_value(&mut self) -> i32 {
        self.value
    }

    fn set_value(&mut self, v: i32) {
        self.value = v;
    }
}

pub fn basic_constructor() -> Box<dyn crate::api::BasicClass> {
    Box::new(Basic { value: 0 })
}

pub struct Receiver;

impl crate::api::ReceiverClass for Receiver {
    fn get_tag(&mut self) -> String {
        "ok".to_string()
    }
}

pub fn receiver_constructor() -> Box<dyn crate::api::ReceiverClass> {
    Box::new(Receiver)
}

