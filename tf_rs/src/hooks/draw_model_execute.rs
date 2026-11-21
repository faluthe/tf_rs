use std::ffi::c_void;

use crate::{
    config::Config,
    features::chams,
    helpers,
    hooks::Hooks,
    interfaces::Interfaces,
    types::{
        ClassId, Color, Entity, Matrix3x4, ModelRenderInfo, Player, material::MATERIAL_VAR_IGNORE_Z,
    },
};

pub extern "C" fn hk_draw_model_execute(
    this: *mut c_void,
    state: *mut c_void,
    info: *const ModelRenderInfo,
    bone_to_world: *mut Matrix3x4,
) {
    let call_original = || {
        Hooks::draw_model_execute()
            .call_draw_model_execute(this, state, info, bone_to_world)
            .expect("Invalid DrawModelExecute function signature");
    };

    let chams = {
        let config = Config::read();
        if !config.esp.master || !config.esp.chams.enabled {
            return call_original();
        }
        (
            config.esp.chams.material,
            config.esp.chams.visible,
            config.esp.chams.occluded,
        )
    };
    let (material_index, visible, occluded) = chams;

    if !Interfaces::engine_client().is_in_game() {
        return call_original();
    }

    let Some(localplayer) = helpers::get_localplayer() else {
        return call_original();
    };

    let Some(info_ref) = (unsafe { info.as_ref() }) else {
        return call_original();
    };

    let Some(entity) = Interfaces::entity_list().get_client_entity::<Entity>(info_ref.entity_index)
    else {
        return call_original();
    };

    if entity.is_dormant() {
        return call_original();
    }

    if !matches!(entity.class_id(), Some(ClassId::Player)) {
        return call_original();
    }

    let player = Player { ent: entity };
    if player.is_dead() {
        return call_original();
    }

    let Some(material) = chams::material(material_index) else {
        return call_original();
    };

    let render_view = Interfaces::render_view();
    let model_render = Interfaces::model_render();

    let old_color = render_view.get_color_modulation();
    let old_blend = render_view.get_blend();

    let draw = |color: Color, ignore_z: bool| {
        render_view.set_color_modulation([
            color.r as f32 / 255.0,
            color.g as f32 / 255.0,
            color.b as f32 / 255.0,
        ]);
        render_view.set_blend(color.a as f32 / 255.0);
        material.set_material_var_flags(MATERIAL_VAR_IGNORE_Z, ignore_z);
        call_original();
    };

    model_render.forced_material_override(Some(&material));
    draw(occluded, true);
    draw(visible, false);
    model_render.forced_material_override(None);

    render_view.set_color_modulation(old_color);
    render_view.set_blend(old_blend);
}
