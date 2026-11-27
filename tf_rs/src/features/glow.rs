//! Outlines around models. Like the game's glows (glow_outline_effect.cpp) the models are
//! stenciled out of the screen and drawn in their colors into the full frame texture. Rather
//! than the game's fixed width outline shader, copies of that texture shifted up to the
//! thickness are laid over the screen around the models, sideways first then up and down

use std::{
    ffi::{CString, c_void},
    sync::{
        OnceLock,
        atomic::{AtomicBool, Ordering},
    },
};

use log::warn;

use crate::{
    config::{Config, GlowConfig, GlowGroupConfig},
    helpers,
    hooks::draw_model_execute::is_aimbot_target,
    interfaces::Interfaces,
    player_db::{self, PlayerCategory},
    types::{
        ClassId, Color, Entity, Material, Player, RenderContext, StencilFunc, StencilOp,
        StencilState, Texture, ViewSetup, entity::handle_index,
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

/// Widest outline in pixels, each pixel costs four more full screen draws
pub const MAX_THICKNESS: i32 = 10;

/// Lays a render target over what's drawn, its black left out and its colors over the rest,
/// so overlapping copies don't add up
fn spread_vmt(texture: &str) -> String {
    format!(
        r#""screenspace_general"
{{
    "$PIXSHADER" "haloadd_ps20"
    "$basetexture" "{texture}"
    "$ALPHA_BLEND_COLOR_OVERLAY" "1"
    "$ignorez" "1"
    "$linearread_basetexture" "1"
    "$linearwrite" "1"
    "$C0_X" "1"
}}"#
    )
}

/// Copies of the full frame texture, and of the one it's spread sideways into
struct Spread {
    horizontal: Material,
    vertical: Material,
}

unsafe impl Send for Spread {}
unsafe impl Sync for Spread {}

static SPREAD: OnceLock<Option<Spread>> = OnceLock::new();

/// Creates the spreading materials, call from the main thread outside of rendering
pub fn init() {
    SPREAD.get_or_init(|| {
        let material_system = Interfaces::material_system();
        let create = |name: &str, texture: &str| {
            let name = CString::new(name).ok()?;
            let vmt = CString::new(spread_vmt(texture)).ok()?;
            material_system.create_material(&name, &vmt)
        };
        let spread = (|| {
            Some(Spread {
                horizontal: create("__glow_spread_h", "_rt_FullFrameFB")?,
                vertical: create("__glow_spread_v", "_rt_FullFrameFB1")?,
            })
        })();
        if spread.is_none() {
            warn!("Failed to create the glow materials");
        }
        spread
    });
}

struct Glowing {
    entity: Entity,
    color: Color,
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
    let Some(Some(spread)) = SPREAD.get() else {
        return;
    };
    let glowing = glowing(&config);
    if glowing.is_empty() {
        return;
    }

    let material_system = Interfaces::material_system();
    let (Some(color), Some(full_frame), Some(full_frame1)) = (
        material_system.find_material("dev/glow_color"),
        material_system.find_texture(c"_rt_FullFrameFB"),
        material_system.find_texture(c"_rt_FullFrameFB1"),
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

    // The models in their colors, whole even where they're behind something
    let (width, height) = (setup.width, setup.height);
    ctx.push_render_target_and_viewport();
    ctx.set_render_target(Some(&full_frame));
    ctx.viewport(0, 0, width, height);
    ctx.clear_color3ub(0, 0, 0);
    ctx.clear_buffers(true, false, false);
    for g in &glowing {
        let a = g.color.a as f32 / 255.0;
        let channel = |v: u8| v as f32 / 255.0 * a;
        render_view.set_color_modulation([
            channel(g.color.r),
            channel(g.color.g),
            channel(g.color.b),
        ]);
        render_view.set_blend(a);
        draw(g.entity);
    }
    ctx.pop_render_target_and_viewport();

    model_render.forced_material_override(None);
    render_view.set_color_modulation(old_color);
    render_view.set_blend(old_blend);
    DRAWING.store(false, Ordering::Relaxed);

    let thickness = config.thickness.clamp(1, MAX_THICKNESS);

    // Spread sideways into the second full frame texture
    ctx.push_render_target_and_viewport();
    ctx.set_render_target(Some(&full_frame1));
    ctx.viewport(0, 0, width, height);
    ctx.clear_color3ub(0, 0, 0);
    ctx.clear_buffers(true, false, false);
    for dx in -thickness..=thickness {
        shifted(
            &ctx,
            &spread.horizontal,
            &full_frame,
            [width, height],
            [dx, 0],
        );
    }
    ctx.pop_render_target_and_viewport();

    // Then up and down onto the screen, around the models
    ctx.set_stencil_state(&OUTSIDE_MODELS);
    let [_, _, screen_w, screen_h] = ctx.get_viewport();
    for dy in -thickness..=thickness {
        shifted(
            &ctx,
            &spread.vertical,
            &full_frame1,
            [screen_w, screen_h],
            [0, dy],
        );
    }
    ctx.set_stencil_state(&StencilState::DISABLED);
}

/// Draw `material` over a `dest` sized viewport, sampling its `source` texture moved by `shift`
/// pixels
fn shifted(
    ctx: &RenderContext,
    material: &Material,
    source: &Texture,
    dest: [i32; 2],
    shift: [i32; 2],
) {
    let [w, h] = dest;
    let [dx, dy] = shift;
    ctx.draw_screen_space_rectangle(
        material,
        [0, 0, w, h],
        [
            dx as f32,
            dy as f32,
            (w - 1 + dx) as f32,
            (h - 1 + dy) as f32,
        ],
        [source.actual_width(), source.actual_height()],
    );
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
            group.enabled.then_some(Glowing {
                entity,
                color: group.color,
            })
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
        self.enabled
    }

    /// The aimbot target's outline in place of this one while it's on
    fn with_target(self, target: &GlowGroupConfig) -> Self {
        if target.enabled { *target } else { self }
    }
}
