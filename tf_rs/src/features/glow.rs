//! Outlines around models, drawn the way the game draws its own (glow_outline_effect.cpp):
//! the models are stenciled out of the screen, drawn in their colors into the full frame
//! texture, and the halo shader outlines what it finds there wherever the stencil is clear

use std::{
    ffi::c_void,
    sync::atomic::{AtomicBool, Ordering},
};

use crate::{
    config::{Config, GlowConfig, GlowGroupConfig},
    helpers,
    hooks::draw_model_execute::is_aimbot_target,
    interfaces::Interfaces,
    player_db::{self, PlayerCategory},
    types::{
        ClassId, Color, Entity, Player, StencilFunc, StencilOp, StencilState, ViewSetup,
        entity::handle_index,
    },
    vfunc,
};

/// Set while the outlined models are drawn, DrawModelExecute leaves them alone then
static DRAWING: AtomicBool = AtomicBool::new(false);

/// Whether the model being drawn is one of the outlines, not the model itself
pub fn drawing() -> bool {
    DRAWING.load(Ordering::Relaxed)
}

/// Every pixel of the models, so the outline only lands around them
const STENCIL_MODELS: StencilState = StencilState {
    enable: true,
    fail: StencilOp::Keep,
    z_fail: StencilOp::Replace,
    pass: StencilOp::Replace,
    func: StencilFunc::Always,
    reference: 1,
    ..StencilState::DISABLED
};

/// Only where no model was stenciled
const OUTSIDE_MODELS: StencilState = StencilState {
    enable: true,
    func: StencilFunc::Equal,
    reference: 0,
    test_mask: 0xFF,
    write_mask: 0,
    ..StencilState::DISABLED
};

const BLACK: Color = Color {
    r: 0,
    g: 0,
    b: 0,
    a: 255,
};

struct Glowing {
    entity: Entity,
    group: GlowGroupConfig,
}

/// Draw the configured outlines over the frame `setup` was drawn with
pub fn render(setup: &ViewSetup) {
    let config = {
        let config = Config::read();
        if !config.esp.master || !config.esp.glow.enabled {
            return;
        }
        config.esp.glow.clone()
    };
    if !Interfaces::engine_client().is_in_game() {
        return;
    }
    let glowing = glowing(&config);
    if glowing.is_empty() {
        return;
    }

    let material_system = Interfaces::material_system();
    let (Some(color), Some(halo), Some(full_frame), Some(quarter)) = (
        material_system.find_material("dev/glow_color"),
        material_system.find_material("dev/halo_add_to_screen"),
        material_system.find_texture(c"_rt_FullFrameFB"),
        material_system.find_texture(c"_rt_SmallFB1"),
    ) else {
        return;
    };
    let Some(ctx) = material_system.render_context() else {
        return;
    };

    let render_view = Interfaces::render_view();
    let model_render = Interfaces::model_render();
    let old_color = render_view.get_color_modulation();
    let old_blend = render_view.get_blend();
    let set_color = |c: Color| {
        let a = c.a as f32 / 255.0;
        let channel = |v: u8| v as f32 / 255.0 * a;
        render_view.set_color_modulation([channel(c.r), channel(c.g), channel(c.b)]);
        render_view.set_blend(a);
    };

    DRAWING.store(true, Ordering::Relaxed);
    model_render.forced_material_override(Some(&color));

    // Stencil the models whole, without touching the color or depth already drawn
    ctx.clear_buffers(false, false, true);
    render_view.set_blend(0.0);
    ctx.override_depth_enable(true, false);
    ctx.set_stencil_state(&STENCIL_MODELS);
    for g in &glowing {
        draw(g.entity);
    }
    ctx.override_depth_enable(false, false);
    ctx.set_stencil_state(&StencilState::DISABLED);

    // The models in their colors, what's behind something in the occluded color and what can
    // be seen in the visible one over it. Black isn't outlined, it hides what can be seen
    ctx.push_render_target_and_viewport();
    ctx.set_render_target(Some(&full_frame));
    ctx.viewport(0, 0, setup.width, setup.height);
    ctx.clear_color3ub(0, 0, 0);
    ctx.clear_buffers(true, false, false);
    for g in &glowing {
        if g.group.occluded {
            set_color(g.group.occluded_color);
            draw(g.entity);
        }
        let visible_color = if g.group.visible {
            g.group.visible_color
        } else {
            BLACK
        };
        // Depth tested, so only what can be seen is drawn over
        ctx.override_depth_enable(true, false);
        set_color(visible_color);
        draw(g.entity);
        ctx.override_depth_enable(false, false);
    }
    ctx.pop_render_target_and_viewport();

    model_render.forced_material_override(None);
    render_view.set_color_modulation(old_color);
    render_view.set_blend(old_blend);
    DRAWING.store(false, Ordering::Relaxed);

    // Outline them on screen. The game samples the full frame texture by its quarter size
    // rectangle over a quarter size target's size, which covers it whole
    if let Some(var) = halo.find_var(c"$C0_X") {
        var.set_float(1.0);
    }
    ctx.set_stencil_state(&OUTSIDE_MODELS);
    let [_, _, width, height] = ctx.get_viewport();
    ctx.draw_screen_space_rectangle(
        &halo,
        [0, 0, width, height],
        [
            0.0,
            -0.5,
            (setup.width / 4 - 1) as f32,
            (setup.height / 4 - 1) as f32,
        ],
        [quarter.actual_width(), quarter.actual_height()],
    );
    ctx.set_stencil_state(&StencilState::DISABLED);
}

/// Each entity with an outline and how it's outlined
fn glowing(config: &GlowConfig) -> Vec<Glowing> {
    let Some(localplayer) = helpers::get_localplayer() else {
        return Vec::new();
    };
    let list = Interfaces::entity_list();
    (1..=list.max_entities())
        .filter_map(|i| {
            let entity = list.get_client_entity::<Entity>(i)?;
            if entity.is_dormant() {
                return None;
            }
            let group = group(i, entity, &localplayer, config)?;
            (group.visible || group.occluded).then_some(Glowing { entity, group })
        })
        .collect()
}

/// How entity `index` is outlined, if it's one that can be
fn group(
    index: i32,
    entity: Entity,
    localplayer: &Player,
    config: &GlowConfig,
) -> Option<GlowGroupConfig> {
    match entity.class_id()? {
        ClassId::Player => {
            let player = Player { ent: entity };
            if player.is_dead() {
                return None;
            }
            if player.ent == localplayer.ent {
                return Some(config.local);
            }
            let category = match player_db::get_by_index(index) {
                PlayerCategory::None => None,
                PlayerCategory::Category1 => Some(config.category1),
                PlayerCategory::Category2 => Some(config.category2),
                PlayerCategory::Category3 => Some(config.category3),
                PlayerCategory::Category4 => Some(config.category4),
            }
            .filter(GlowGroupConfig::active);
            if player.team() == localplayer.team() {
                return Some(category.unwrap_or(config.friendly));
            }
            let group = category.unwrap_or(config.enemy);
            Some(if is_aimbot_target(index) {
                group.with_target(&config.aimbot_target)
            } else {
                group
            })
        }
        ClassId::Sentry | ClassId::Dispenser | ClassId::Teleporter => {
            Some(if entity.team() == localplayer.team() {
                config.building_friendly
            } else {
                config.building_enemy
            })
        }
        _ => None,
    }
}

/// Offsets of the move child and move peer handles, an entity's attachments are its move
/// children, its weapon and cosmetics for a player
const MOVE_CHILD: usize = 0x220;
const MOVE_PEER: usize = 0x224;

/// Draw `entity` and what's attached to it, like the game's glows
fn draw(entity: Entity) {
    if !should_draw(entity) {
        return;
    }
    draw_model(entity);
    let mut child = attached(entity, MOVE_CHILD);
    while let Some(c) = child {
        if should_draw(c) {
            draw_model(c);
        }
        child = attached(c, MOVE_PEER);
    }
}

/// The entity the handle at `offset` in `entity` points at
fn attached(entity: Entity, offset: usize) -> Option<Entity> {
    let handle = unsafe { *entity.field::<u32>(offset) };
    if handle == u32::MAX || handle == 0 {
        return None;
    }
    let found = Interfaces::entity_list().get_client_entity::<Entity>(handle_index(handle))?;
    (found.ref_handle() == handle).then_some(found)
}

fn should_draw(entity: Entity) -> bool {
    let f = vfunc!(entity.vtable, 136, (*mut c_void) -> bool);
    f(entity.this) && !entity.is_dormant()
}

/// C_BaseEntity::DrawModel with STUDIO_RENDER
fn draw_model(entity: Entity) {
    let f = vfunc!(entity.vtable, 56, (*mut c_void, i32) -> i32);
    f(entity.this, 1);
}

impl GlowGroupConfig {
    pub fn active(&self) -> bool {
        self.visible || self.occluded
    }

    /// The aimbot target's outline on top of this one, in its colors where it's on
    fn with_target(mut self, target: &GlowGroupConfig) -> Self {
        if target.visible {
            self.visible = true;
            self.visible_color = target.visible_color;
        }
        if target.occluded {
            self.occluded = true;
            self.occluded_color = target.occluded_color;
        }
        self
    }
}
