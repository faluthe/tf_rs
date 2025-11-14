#[repr(C)]
pub struct PlayerInfo {
    pub name: [u8; 32],
    user_id: i32,
    guid: [u8; 33],
    friends_id: u64,
    friends_name: [u8; 32],
    fake_player: bool,
    is_hltv: bool,
    custom_files: [u64; 4],
    files_downloaded: u8,
}

impl Default for PlayerInfo {
    fn default() -> Self {
        PlayerInfo {
            name: [0; 32],
            user_id: 0,
            guid: [0; 33],
            friends_id: 0,
            friends_name: [0; 32],
            fake_player: false,
            is_hltv: false,
            custom_files: [0; 4],
            files_downloaded: 0,
        }
    }
}
