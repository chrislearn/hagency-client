//! Personal publishing on Matrix. Never shares a room with File Transfer.
pub mod backend;
pub mod dm_sharing;
pub mod model;
pub mod ui;
pub mod window;
#[cfg(test)]
mod live_tests;

pub const ROOM_TYPE: &str = "rs.robius.robrix.moments";
pub const ACCOUNT_DATA: &str = "rs.robius.robrix.moments.preferences";

pub fn is_moments(room: &matrix_sdk::Room) -> bool {
    room.room_type()
        .is_some_and(|kind| kind.as_str() == ROOM_TYPE)
}

pub fn script_mod(vm: &mut makepad_widgets::ScriptVm) {
    ui::script_mod(vm);
    window::script_mod(vm);
}
