pub(crate) mod camera;
mod alarm;
mod accelerometer;
pub(crate) mod rumble;

use std::ffi::CString;
use std::sync::atomic::Ordering;
use sameboy_sys::{GB_gbs_switch_track, GB_get_rom_crc32, GB_get_rom_title, GB_load_battery_from_buffer, GB_load_gbs_from_buffer, GB_load_rom_from_buffer, GB_save_battery_size, GB_save_battery_to_buffer};
use super::Gameboy;
use super::gbs_info::GbsInfo;
use anyhow::{bail, Result};

impl Gameboy {
    /// Load a cartridge ROM.
    pub fn load_rom(&mut self, rom: &[u8]) {
        unsafe {
            (*self.inner_mut()).boot_rom_unmapped.store(false, Ordering::Release);
            GB_load_rom_from_buffer(self.as_mut_ptr(), rom.as_ptr(), rom.len());
        }
    }

    /// Load a cartridge battery save.
    pub fn load_sram(&mut self, sram: &[u8]) {
        unsafe {
            GB_load_battery_from_buffer(self.as_mut_ptr(), sram.as_ptr(), sram.len());
        }
    }

    /// Retrieve a cartridge battery save.
    pub fn save_sram(&mut self) -> Vec<u8> {
        unsafe {
            let sram_size = GB_save_battery_size(self.as_mut_ptr()) as usize;
            let mut result = vec![0u8; sram_size];
            GB_save_battery_to_buffer(self.as_mut_ptr(), result.as_mut_ptr(), result.len());
            result
        }
    }

    /// Load a Game Boy Sound module.
    pub fn load_gbs(&mut self, gbs: &[u8]) -> Result<GbsInfo> {
        let mut info = GbsInfo::new();
        unsafe {
            (*self.inner_mut()).boot_rom_unmapped.store(false, Ordering::Release);

            let ret = GB_load_gbs_from_buffer(self.as_mut_ptr(), gbs.as_ptr(), gbs.len(), info.as_mut_ptr());
            if ret != 0 {
                bail!("Invalid GBS file.");
            }
        }
        Ok(info)
    }

    /// Select the currently playing track in a GBS module.
    pub fn gbs_change_track(&mut self, track: u8) {
        unsafe {
            GB_gbs_switch_track(self.as_mut_ptr(), track);
        }
    }

    /// Read the game title from the ROM header.
    pub fn game_title(&mut self) -> Result<String> {
        unsafe {
            let mut title_bytes = vec![0u8; 16];
            GB_get_rom_title(self.as_mut_ptr(), title_bytes.as_mut_ptr() as *mut _);

            if let Some(terminator) = title_bytes.iter().position(|&b| b == 0) {
                title_bytes.truncate(terminator);
            }
            title_bytes.push(0);

            Ok(CString::from_vec_with_nul(title_bytes)?.into_string()?)
        }
    }

    /// Calculate the CRC32 checksum value of the loaded ROM.
    pub fn rom_crc32(&mut self) -> u32 {
        unsafe {
            GB_get_rom_crc32(self.as_mut_ptr())
        }
    }
}
