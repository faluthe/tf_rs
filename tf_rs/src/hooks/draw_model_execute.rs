use std::ffi::c_void;

use crate::{
    config::{ChamsConfig, ChamsGroupConfig, Config},
    features::{backtrack, chams},
    helpers,
    hooks::Hooks,
    interfaces::Interfaces,
    player_db::{self, PlayerCategory},
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

    let chams_config = {
        let config = Config::read();
        if !config.esp.master || !config.esp.chams.enabled {
            return call_original();
        }
        config.esp.chams.clone()
    };

    if !Interfaces::engine_client().is_in_game() {
        return call_original();
    }

    let Some(info_ref) = (unsafe { info.as_ref() }) else {
        return call_original();
    };

    draw_backtrack(info_ref, &chams_config, |bones| {
        Hooks::draw_model_execute()
            .call_draw_model_execute(this, state, info, bones)
            .expect("Invalid DrawModelExecute function signature");
    });

    let Some(group) = chams_group(info_ref, &chams_config).filter(|g| g.active()) else {
        return call_original();
    };

    let Some((layers, materials)) = chams::layers(group.material) else {
        return call_original();
    };

    let render_view = Interfaces::render_view();
    let model_render = Interfaces::model_render();

    let old_color = render_view.get_color_modulation();
    let old_blend = render_view.get_blend();

    let set_color = |color: Color| {
        render_view.set_color_modulation([
            color.r as f32 / 255.0,
            color.g as f32 / 255.0,
            color.b as f32 / 255.0,
        ]);
        render_view.set_blend(color.a as f32 / 255.0);
    };
    let restore_color = || {
        render_view.set_color_modulation(old_color);
        render_view.set_blend(old_blend);
    };
    let call_original_normal = || {
        model_render.forced_material_override(None);
        call_original();
    };

    // Every layer drawn on top of the last, each in its own drifting shade of `color`
    let time = Interfaces::global_vars().realtime;
    let draw_layers = |color: Color, ignore_z: bool| {
        for (layer, material) in layers.iter().zip(materials) {
            model_render.forced_material_override(Some(material));
            material.set_material_var_flags(MATERIAL_VAR_IGNORE_Z, ignore_z);
            set_color(layer.color(color, time));
            call_original();
        }
    };

    if group.occluded {
        draw_layers(group.occluded_color, true);
    }

    if group.visible {
        draw_layers(group.visible_color, false);
    } else {
        // Normal model wherever the entity can be seen
        restore_color();
        call_original_normal();
    }

    model_render.forced_material_override(None);
    for material in materials {
        material.set_material_var_flags(MATERIAL_VAR_IGNORE_Z, false);
    }
    restore_color();
}

/// Draw the model posed at each backtrack record of an enemy player, `draw` renders it with
/// the given bones
fn draw_backtrack(info: &ModelRenderInfo, config: &ChamsConfig, draw: impl Fn(*mut Matrix3x4)) {
    let group = config.backtrack;
    if !group.active() || !is_enemy_player(info.entity_index) {
        return;
    }
    let Some((layers, materials)) = chams::layers(group.material) else {
        return;
    };

    let render_view = Interfaces::render_view();
    let model_render = Interfaces::model_render();
    let old_color = render_view.get_color_modulation();
    let old_blend = render_view.get_blend();
    let time = Interfaces::global_vars().realtime;

    backtrack::valid_bones(info.entity_index, |bones| {
        let passes = [
            (group.occluded, group.occluded_color, true),
            (group.visible, group.visible_color, false),
        ];
        for (_, color, ignore_z) in passes.into_iter().filter(|p| p.0) {
            for (layer, material) in layers.iter().zip(materials) {
                let color = layer.color(color, time);
                model_render.forced_material_override(Some(material));
                material.set_material_var_flags(MATERIAL_VAR_IGNORE_Z, ignore_z);
                render_view.set_color_modulation([
                    color.r as f32 / 255.0,
                    color.g as f32 / 255.0,
                    color.b as f32 / 255.0,
                ]);
                render_view.set_blend(color.a as f32 / 255.0);
                // The engine only reads the bones, the cast is for the signature
                draw(bones.as_ptr() as *mut Matrix3x4);
            }
        }
    });

    model_render.forced_material_override(None);
    for material in materials {
        material.set_material_var_flags(MATERIAL_VAR_IGNORE_Z, false);
    }
    render_view.set_color_modulation(old_color);
    render_view.set_blend(old_blend);
}

fn is_enemy_player(index: i32) -> bool {
    let Some(localplayer) = helpers::get_localplayer() else {
        return false;
    };
    Interfaces::entity_list()
        .get_client_entity::<Player>(index)
        .is_some_and(|p| {
            p.ent
                .class_id()
                .is_some_and(|c| matches!(c, ClassId::Player))
                && !p.is_dormant()
                && !p.is_dead()
                && p.team() != localplayer.team()
        })
}

/// Which chams configuration applies to the model being drawn, if any
fn chams_group(info: &ModelRenderInfo, config: &ChamsConfig) -> Option<ChamsGroupConfig> {
    // Client only entities have no index, the first person weapon is one of them
    if info.entity_index < 1 {
        let model_info = Interfaces::model_info();
        let is_weapon = model_info.get_model_name(info.model).contains("weapons/");
        return is_weapon.then_some(config.weapon);
    }

    let entity = Interfaces::entity_list().get_client_entity::<Entity>(info.entity_index)?;
    if entity.is_dormant() {
        return None;
    }

    let localplayer = helpers::get_localplayer()?;

    match entity.class_id() {
        Some(ClassId::Player) => {
            let player = Player { ent: entity };
            if player.is_dead() {
                return None;
            }

            if player.ent == localplayer.ent {
                return Some(config.local);
            }

            let player_info = Interfaces::engine_client().get_player_info(info.entity_index);
            let guid = str::from_utf8(&player_info.guid)
                .unwrap_or("")
                .trim_end_matches('\0');
            let category = match player_db::get(guid) {
                PlayerCategory::None => None,
                PlayerCategory::Category1 => Some(config.category1),
                PlayerCategory::Category2 => Some(config.category2),
                PlayerCategory::Category3 => Some(config.category3),
                PlayerCategory::Category4 => Some(config.category4),
            };
            if let Some(category) = category.filter(|c| c.active()) {
                return Some(category);
            }

            if player.team() == localplayer.team() {
                Some(config.friendly)
            } else {
                Some(config.enemy)
            }
        }
        Some(ClassId::Sentry | ClassId::Dispenser | ClassId::Teleporter) => {
            if entity.team() == localplayer.team() {
                Some(config.building_friendly)
            } else {
                Some(config.building_enemy)
            }
        }
        _ if entity.class_name() == "CTFViewModel" => Some(config.hands),
        _ => None,
    }
}
