use crate::api::{ColorEnum, MsgStruct};

pub struct DefaultTestStructEnumSingleton;

impl crate::api::TestStructEnumSingleton for DefaultTestStructEnumSingleton {
    fn get_red(&self) -> ColorEnum {
        ColorEnum::Red
    }

    fn make_msg(&self, id: i32, text: String) -> MsgStruct {
        MsgStruct { id, text }
    }
}

#[no_mangle]
pub extern "C" fn ridl_create_test_struct_enum_singleton() -> *mut dyn crate::api::TestStructEnumSingleton {
    Box::into_raw(Box::new(DefaultTestStructEnumSingleton))
}
