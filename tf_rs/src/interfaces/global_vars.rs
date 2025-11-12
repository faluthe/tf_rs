#[derive(Clone, Copy)]
pub struct GlobalVars {
    _realtime: f32,
    _framecount: i32,
    _absoluteframetime: f32,
    _curtime: f32,
    _frametime: f32,
    _maxclients: i32,
    _tickcount: i32,
    pub interval_per_tick: f32,
    _interpolation_amount: f32,
    _sim_ticks_this_frame: i32,
    _network_protocol: i32,
}
