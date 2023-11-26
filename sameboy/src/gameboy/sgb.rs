use std::sync::{Arc, Mutex};
use sameboy_sys::{GB_gameboy_t, GB_get_clock_rate, GB_get_player_count, GB_get_unmultiplied_clock_rate, GB_set_clock_multiplier, GB_set_icd_hreset_callback, GB_set_icd_pixel_callback, GB_set_icd_vreset_callback, GB_set_joyp_write_callback};
use super::Gameboy;  // Insert Xzibit photo here
use super::inner::Dummy;

extern fn joyp_write_callback(gb: *mut GB_gameboy_t, value: u8) {
    unsafe {
        let mut gb = Gameboy::wrap(gb);
        let id = gb.id();

        (*gb.inner_mut())
            .sgb_receiver
            .lock()
            .unwrap()
            .joypad_write(id, value);
    }
}

extern fn icd_pixel_callback(gb: *mut GB_gameboy_t, row: u8) {
    unsafe {
        let mut gb = Gameboy::wrap(gb);
        let id = gb.id();

        (*gb.inner_mut())
            .sgb_receiver
            .lock()
            .unwrap()
            .icd_pixel(id, row);
    }
}

extern fn icd_hreset_callback(gb: *mut GB_gameboy_t) {
    unsafe {
        let mut gb = Gameboy::wrap(gb);
        let id = gb.id();

        (*gb.inner_mut())
            .sgb_receiver
            .lock()
            .unwrap()
            .icd_hreset(id);
    }
}

extern fn icd_vreset_callback(gb: *mut GB_gameboy_t) {
    unsafe {
        let mut gb = Gameboy::wrap(gb);
        let id = gb.id();

        (*gb.inner_mut())
            .sgb_receiver
            .lock()
            .unwrap()
            .icd_vreset(id);
    }
}

pub trait SuperGameboyReceiver {
    fn joypad_write(&mut self, id: usize, value: u8);
    fn icd_pixel(&mut self, id: usize, row: u8);
    fn icd_hreset(&mut self, id: usize);
    fn icd_vreset(&mut self, id: usize);
}

impl Gameboy {
    pub(crate) unsafe fn init_sgb(gb: *mut GB_gameboy_t) {
        GB_set_joyp_write_callback(gb, Some(joyp_write_callback));
        GB_set_icd_pixel_callback(gb, Some(icd_pixel_callback));
        GB_set_icd_hreset_callback(gb, Some(icd_hreset_callback));
        GB_set_icd_vreset_callback(gb, Some(icd_vreset_callback));
    }

    pub(crate) unsafe fn deinit_sgb(gb: *mut GB_gameboy_t) {
        GB_set_joyp_write_callback(gb, None);
        GB_set_icd_pixel_callback(gb, None);
        GB_set_icd_hreset_callback(gb, None);
        GB_set_icd_vreset_callback(gb, None);
    }
}

impl Gameboy {
    pub fn set_sgb_receiver(&mut self, sgb_receiver: Option<Arc<Mutex<dyn SuperGameboyReceiver>>>) {
        unsafe {
            // Some things (e.g. the joypad) act differently when these callbacks
            // are set, so only set them when we need them so we can anticipate
            // the behavior changes.
            if sgb_receiver.is_some() {
                Self::init_sgb(self.as_mut_ptr());
            } else {
                Self::deinit_sgb(self.as_mut_ptr());
            }

            (*self.inner_mut()).sgb_receiver = sgb_receiver.unwrap_or(Arc::new(Mutex::new(Dummy)));
        }
    }

    /// Get the clock rate of the Game Boy.
    pub fn clock_rate(&mut self) -> u32 {
        unsafe {
            GB_get_clock_rate(self.as_mut_ptr())
        }
    }

    /// Get the clock rate of the Game Boy without the effects
    /// of the clock multiplier.
    pub fn unmultiplied_clock_rate(&mut self) -> u32 {
        unsafe {
            GB_get_unmultiplied_clock_rate(self.as_mut_ptr())
        }
    }

    /// Configure the clock multiplier.
    pub fn set_clock_multiplier(&mut self, clock_multiplier: f64) {
        unsafe {
            GB_set_clock_multiplier(self.as_mut_ptr(), clock_multiplier);
        }
    }

    /// Get the number of SGB players.
    pub fn player_count(&mut self) -> u32 {
        unsafe {
            GB_get_player_count(self.as_mut_ptr()) as u32
        }
    }
}
