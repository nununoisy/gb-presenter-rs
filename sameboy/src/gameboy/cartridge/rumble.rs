use std::sync::{Arc, Mutex};
use sameboy_sys::{GB_gameboy_t, GB_rumble_mode_t, GB_rumble_mode_t_GB_RUMBLE_ALL_GAMES, GB_rumble_mode_t_GB_RUMBLE_CARTRIDGE_ONLY, GB_rumble_mode_t_GB_RUMBLE_DISABLED, GB_set_rumble_callback, GB_set_rumble_mode};
use super::super::Gameboy;

extern fn rumble_callback(gb: *mut GB_gameboy_t, amplitude: f64) {
    unsafe {
        let mut gb = Gameboy::wrap(gb);
        let id = gb.id();

        (*gb.inner_mut())
            .rumble_receiver
            .clone()
            .map(|r| r.lock().unwrap().receive(id, amplitude));
    }
}

pub trait RumbleReceiver {
    /// Called when the rumble amplitude changes.
    fn receive(&mut self, id: usize, amplitude: f64);
}

#[derive(Debug, Copy, Clone)]
pub enum RumbleMode {
    /// Rumble is disabled.
    Disabled,
    /// Rumble is enabled for cartridges that have rumble hardware.
    RumbleCartsOnly,
    /// Rumble is always enabled even if the cartridge doesn't have it.
    AllGames
}

impl From<GB_rumble_mode_t> for RumbleMode {
    fn from(value: GB_rumble_mode_t) -> Self {
        match value {
            GB_rumble_mode_t_GB_RUMBLE_DISABLED => Self::Disabled,
            GB_rumble_mode_t_GB_RUMBLE_CARTRIDGE_ONLY => Self::RumbleCartsOnly,
            GB_rumble_mode_t_GB_RUMBLE_ALL_GAMES => Self::AllGames,
            _ => unreachable!("Invalid GB_rumble_mode_t value")
        }
    }
}

impl From<RumbleMode> for GB_rumble_mode_t {
    fn from(value: RumbleMode) -> Self {
        match value {
            RumbleMode::Disabled => GB_rumble_mode_t_GB_RUMBLE_DISABLED,
            RumbleMode::RumbleCartsOnly => GB_rumble_mode_t_GB_RUMBLE_CARTRIDGE_ONLY,
            RumbleMode::AllGames => GB_rumble_mode_t_GB_RUMBLE_ALL_GAMES
        }
    }
}

impl Gameboy {
    pub(crate) unsafe fn init_rumble(gb: *mut GB_gameboy_t) {
        GB_set_rumble_callback(gb, Some(rumble_callback));
    }
}

impl Gameboy {
    pub fn set_rumble_receiver(&mut self, rumble_receiver: Option<Arc<Mutex<dyn RumbleReceiver>>>) {
        unsafe {
            (*self.inner_mut()).rumble_receiver = rumble_receiver;
        }
    }

    pub fn set_rumble_mode(&mut self, rumble_mode: RumbleMode) {
        unsafe {
            GB_set_rumble_mode(self.as_mut_ptr(), rumble_mode.into());
        }
    }
}
