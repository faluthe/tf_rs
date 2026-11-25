#[derive(Clone, Copy)]
pub struct GlobalVars {
    pub realtime: f32,
    _framecount: i32,
    _absoluteframetime: f32,
    pub curtime: f32,
    _frametime: f32,
    _maxclients: i32,
    pub tickcount: i32,
    pub interval_per_tick: f32,
    _interpolation_amount: f32,
    _sim_ticks_this_frame: i32,
    _network_protocol: i32,
}

impl GlobalVars {
    /// Nearest tick to `time`
    pub fn time_to_ticks(&self, time: f32) -> i32 {
        (0.5 + time / self.interval_per_tick) as i32
    }

    pub fn ticks_to_time(&self, ticks: i32) -> f32 {
        self.interval_per_tick * ticks as f32
    }
}
