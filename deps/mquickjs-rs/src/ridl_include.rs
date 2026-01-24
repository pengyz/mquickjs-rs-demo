#[macro_export]
macro_rules! ridl_include_api {
    () => {
        include!(concat!(env!("OUT_DIR"), "/api.rs"));
    };
}

#[macro_export]
macro_rules! ridl_include_glue {
    () => {
        include!(concat!(env!("OUT_DIR"), "/glue.rs"));
    };
}

#[macro_export]
macro_rules! ridl_include_module {
    () => {
        pub mod api {
            $crate::ridl_include_api!();
        }

        // Intentionally include glue at crate root so app-side aggregation can reference symbols
        // like `crate::RIDL_*_SINGLETON_VT` without users having to re-export from an internal
        // `glue` module.
        $crate::ridl_include_glue!();
    };
}
