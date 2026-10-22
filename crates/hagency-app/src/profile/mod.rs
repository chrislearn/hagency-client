use makepad_widgets::ScriptVm;

pub mod user_profile;
pub mod user_profile_cache;
pub mod agent_details;

pub fn script_mod(vm: &mut ScriptVm) {
    user_profile::script_mod(vm);
    agent_details::script_mod(vm);
}
