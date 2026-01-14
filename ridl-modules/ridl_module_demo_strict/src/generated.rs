pub mod api {
    include!(concat!(env!("OUT_DIR"), "/api.rs"));
}

pub mod glue {
    include!(concat!(env!("OUT_DIR"), "/glue.rs"));
}
