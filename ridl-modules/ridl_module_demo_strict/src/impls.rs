pub fn sayhello2() -> String {
    "hello2".to_string()
}

pub fn strict_sum_i32(nums: Vec<i32>) -> i32 {
    nums.into_iter().sum()
}

pub fn strict_count_args(args: Vec<mquickjs_rs::ValueRef<'_>>) -> i32 {
    args.len() as i32
}

pub fn strict_add_i32(a: i32, b: i32) -> i32 {
    a + b
}

pub fn strict_echo_str(s: *const std::os::raw::c_char) -> String {
    if s.is_null() {
        return "".to_string();
    }
    unsafe { std::ffi::CStr::from_ptr(s).to_string_lossy().into_owned() }
}
