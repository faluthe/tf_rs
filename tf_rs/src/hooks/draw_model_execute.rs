use std::ffi::c_void;

use crate::{
    config::{ChamsConfig, ChamsGroupConfig, ChamsLayerConfig, ChamsTargetConfig, Config},
    features::{
        backtrack,
        chams::{self, Loaded},
        preview,
    },
    globals::Globals,
    helpers,
    hooks::Hooks,
    interfaces::Interfaces,
    player_db::{self, PlayerCategory},
    types::{
        ClassId, Color, Entity, Matrix3x4, ModelRenderInfo, Player,
        material::MATERIAL_VAR_IGNORE_Z, player::Bones,
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

    let (chams_config, backtrack) = {
        let config = Config::read();
        if !config.esp.master || !config.esp.chams.enabled {
            return call_original();
        }
        (config.esp.chams.clone(), config.backtrack.enabled)
    };

    if !Interfaces::engine_client().is_in_game() {
        return call_original();
    }

    let Some(info_ref) = (unsafe { info.as_ref() }) else {
        return call_original();
    };

    let preview = preview::drawing();
    let backtrack = backtrack
        && helpers::get_localplayer()
            .is_some_and(|local| backtrack::weapon_is_lag_compensated(&local));
    if backtrack && preview.is_none() {
        draw_backtrack(info_ref, &chams_config, |bones| {
            Hooks::draw_model_execute()
                .call_draw_model_execute(this, state, info, bones)
                .expect("Invalid DrawModelExecute function signature");
        });
    }

    if let Some(subject) = preview {
        let occluded = preview::state().occluded;
        let (layers, normal_fallback) = preview::chams_layers(subject, occluded, &chams_config);
        return draw_layers(&layers, normal_fallback, WORLD_MODEL_SCALE, call_original);
    }

    let Some(group) = chams_group(info_ref, &chams_config).filter(|g| g.active()) else {
        return call_original();
    };
    let scale = if is_first_person(info_ref) {
        1.0
    } else {
        WORLD_MODEL_SCALE
    };
    draw_layers(&group.layers(), true, scale, call_original);
}

/// How much more scaled textures repeat on models in the world than on the first person ones.
/// Player models fit the whole body in one texture, so a tile would cover most of them
pub(crate) const WORLD_MODEL_SCALE: f32 = 4.0;

fn is_first_person(info: &ModelRenderInfo) -> bool {
    info.entity_index < 1
        || Interfaces::entity_list()
            .get_client_entity::<Entity>(info.entity_index)
            .is_some_and(|e| e.class_name() == "CTFViewModel")
}

/// Draw the model once per pass of each layer with `draw`, every occluded pass first so the
/// visible ones land on top of them. With `normal_fallback` the normal model is drawn where it
/// can be seen when no layer has a visible pass. Scaled textures are scaled up by `scale`
pub(crate) fn draw_layers(
    layers: &[ChamsLayerConfig],
    normal_fallback: bool,
    scale: f32,
    draw: impl Fn(),
) {
    let render_view = Interfaces::render_view();
    let model_render = Interfaces::model_render();
    let old_color = render_view.get_color_modulation();
    let old_blend = render_view.get_blend();
    let time = Interfaces::global_vars().realtime;

    let set_color = |color: Color| {
        render_view.set_color_modulation([
            color.r as f32 / 255.0,
            color.g as f32 / 255.0,
            color.b as f32 / 255.0,
        ]);
        render_view.set_blend(color.a as f32 / 255.0);
    };
    let draw_normal = || {
        model_render.forced_material_override(None);
        render_view.set_color_modulation(old_color);
        render_view.set_blend(old_blend);
        draw();
    };

    let mut used: Vec<&'static [Loaded]> = Vec::new();
    // Each of the material's own layers in its drifting shade of `color`
    let mut draw_pass = |material: usize, color: Color, ignore_z: bool| {
        let Some((sublayers, loaded)) = chams::layers(material) else {
            return;
        };
        used.push(loaded);
        for (sublayer, loaded) in sublayers.iter().zip(loaded) {
            let color = sublayer.color(color, time);
            model_render.forced_material_override(Some(&loaded.material));
            loaded
                .material
                .set_material_var_flags(MATERIAL_VAR_IGNORE_Z, ignore_z);
            loaded.tint(color);
            loaded.scale(sublayer.scale * scale);
            set_color(color);
            draw();
        }
    };

    // The model's own materials can't be drawn through walls
    for layer in layers
        .iter()
        .filter(|l| l.occluded && !chams::is_original(l.material))
    {
        draw_pass(layer.material, layer.occluded_color, true);
    }
    if normal_fallback && !layers.iter().any(|l| l.visible) {
        draw_normal();
    }
    for layer in layers.iter().filter(|l| l.visible) {
        if chams::is_original(layer.material) {
            draw_normal();
        } else {
            draw_pass(layer.material, layer.visible_color, false);
        }
    }

    model_render.forced_material_override(None);
    for loaded in used.into_iter().flatten() {
        loaded
            .material
            .set_material_var_flags(MATERIAL_VAR_IGNORE_Z, false);
    }
    render_view.set_color_modulation(old_color);
    render_view.set_blend(old_blend);
}

/// Draw the model posed at each backtrack record of an enemy player, `draw` renders it with
/// the given bones
fn draw_backtrack(info: &ModelRenderInfo, config: &ChamsConfig, draw: impl Fn(*mut Matrix3x4)) {
    // The target falls back to the other ticks' look when it has none of its own
    let target_group = if config.backtrack_target.active() {
        config.backtrack_target
    } else {
        config.backtrack
    };
    if !(config.backtrack.active() || target_group.active()) || !is_enemy_player(info.entity_index)
    {
        return;
    }
    let Some(records) = backtrack::drawn(info.entity_index, info.origin) else {
        return;
    };

    // The engine only reads the bones, the cast is for the signature
    let draw = |bones: &Bones| draw(bones.as_ptr() as *mut Matrix3x4);
    // The target last so it's drawn over the others, each layer set once for all its records
    if records.others().next().is_some() {
        draw_layers(&config.backtrack.layers(), false, WORLD_MODEL_SCALE, || {
            records.others().for_each(draw)
        });
    }
    if records.target().is_some() {
        draw_layers(&target_group.layers(), false, WORLD_MODEL_SCALE, || {
            records.target().into_iter().for_each(draw)
        });
    }
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
            if player.team() == localplayer.team() {
                return category.filter(|c| c.active()).or(Some(config.friendly));
            }
            let group = category.filter(|c| c.active()).unwrap_or(config.enemy);
            Some(if is_aimbot_target(info.entity_index) {
                recolor(group, &config.aimbot_target)
            } else {
                group
            })
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

fn is_aimbot_target(index: i32) -> bool {
    // Never wait on the aimbot while drawing
    Globals::try_read().is_some_and(|g| g.target.as_ref().is_some_and(|t| t.target_index == index))
}

/// `group` with its layers in the target's colors, each keeping its own transparency
pub(crate) fn recolor(mut group: ChamsGroupConfig, target: &ChamsTargetConfig) -> ChamsGroupConfig {
    let tint = |color: Color, layer: Color| Color {
        a: (layer.a as u16 * color.a as u16 / 255) as u8,
        ..color
    };
    for i in 0..ChamsGroupConfig::LAYERS {
        let layer = group.layer_mut(i);
        if target.visible {
            *layer.visible_color = tint(target.visible_color, *layer.visible_color);
        }
        if target.occluded {
            *layer.occluded_color = tint(target.occluded_color, *layer.occluded_color);
        }
    }
    group
}
