use crate::types::Vec3;

/// CMoveData, what game movement reads a command from and writes its results to
#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct MoveData {
    /// m_bFirstRunOfFunctions and m_bGameCodeMovedPlayer bits
    pub run_flags: u8,
    pub player_handle: u32,
    pub impulse: i32,
    pub view_angles: Vec3,
    pub abs_view_angles: Vec3,
    pub buttons: i32,
    pub old_buttons: i32,
    pub forward_move: f32,
    pub old_forward_move: f32,
    pub side_move: f32,
    pub up_move: f32,
    pub max_speed: f32,
    pub client_max_speed: f32,
    pub velocity: Vec3,
    pub angles: Vec3,
    pub old_angles: Vec3,
    pub out_step_height: f32,
    pub out_wish_vel: Vec3,
    pub out_jump_vel: Vec3,
    pub constraint_center: Vec3,
    pub constraint_radius: f32,
    pub constraint_width: f32,
    pub constraint_speed_factor: f32,
    pub abs_origin: Vec3,
}

const _: () = assert!(std::mem::size_of::<MoveData>() == 168);
