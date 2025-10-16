#![allow(non_snake_case)]
#![allow(non_upper_case_globals)]
#![allow(dead_code)] // TODO: actually trim down header using bindgen options
#![allow(non_camel_case_types)]
include!(concat!(env!("OUT_DIR"), "/nuklear_bindings.rs"));
