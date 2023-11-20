use std::ptr;
use super::Gameboy;
use super::model::Model;
use anyhow::{Result, bail};
use sameboy_sys::{GB_get_save_state_size, GB_get_state_model_from_buffer, GB_load_state_from_buffer, GB_model_t, GB_model_t_GB_MODEL_DMG_B, GB_save_state_to_buffer};

impl Gameboy {
    /// Save the current state of the console to a buffer.
    pub fn save_state(&mut self) -> Vec<u8> {
        unsafe {
            let buf_size = GB_get_save_state_size(self.as_mut_ptr());
            let mut buf = vec![0u8; buf_size];
            GB_save_state_to_buffer(self.as_mut_ptr(), buf.as_mut_ptr());
            buf
        }
    }

    /// Load a save state from a buffer.
    pub fn load_state(&mut self, save_state: &[u8]) -> Result<()> {
        unsafe {
            let ret = GB_load_state_from_buffer(self.as_mut_ptr(), save_state.as_ptr(), save_state.len());
            if ret != 0 {
                bail!("Failed to load save state! ({})", ret);
            }
            Ok(())
        }
    }

    /// Get the Game Boy model of a save state.
    pub fn save_state_model(save_state: &[u8]) -> Model {
        unsafe {
            let mut model: GB_model_t = GB_model_t_GB_MODEL_DMG_B;
            GB_get_state_model_from_buffer(save_state.as_ptr(), save_state.len(), ptr::addr_of_mut!(model));
            model.into()
        }
    }
}
