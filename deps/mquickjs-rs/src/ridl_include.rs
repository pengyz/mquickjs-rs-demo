#[macro_export]
macro_rules! ridl_include_glue {
    () => {
        include!(concat!(env!("OUT_DIR"), "/glue.rs"));
    };
}
