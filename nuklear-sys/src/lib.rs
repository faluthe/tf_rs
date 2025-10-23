#![allow(
    non_snake_case,
    non_upper_case_globals,
    dead_code,
    non_camel_case_types
)] // TODO: actually trim down header using bindgen options
include!(concat!(env!("OUT_DIR"), "/nuklear_bindings.rs"));
