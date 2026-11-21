use std::ffi::c_void;

use crate::{
    hooks::Hooks,
    types::{Matrix3x4, ModelRenderInfo},
};

pub extern "C" fn hk_draw_model_execute(
    this: *mut c_void,
    state: *mut c_void,
    info: *const ModelRenderInfo,
    bone_to_world: *mut Matrix3x4,
) {
    Hooks::draw_model_execute()
        .call_draw_model_execute(this, state, info, bone_to_world)
        .expect("Invalid DrawModelExecute function signature");
}
