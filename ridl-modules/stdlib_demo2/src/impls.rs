pub fn sayhello2() -> String {
    "hello2".to_string()
}

pub fn sum_i32(nums: Vec<i32>) -> i32 {
    nums.into_iter().sum()
}

pub fn count_args(args: Vec<mquickjs_rs::mquickjs_ffi::JSValue>) -> i32 {
    args.len() as i32
}
