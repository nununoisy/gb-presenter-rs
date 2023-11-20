use sameboy_sys::{GB_gameboy_t, GB_load_boot_rom_from_buffer};
use super::Gameboy;

#[cfg(feature = "include-bootroms")]
use sameboy_sys::{GB_boot_rom_t, GB_boot_rom_t_GB_BOOT_ROM_DMG_0, GB_boot_rom_t_GB_BOOT_ROM_DMG, GB_boot_rom_t_GB_BOOT_ROM_MGB, GB_boot_rom_t_GB_BOOT_ROM_SGB, GB_boot_rom_t_GB_BOOT_ROM_SGB2, GB_boot_rom_t_GB_BOOT_ROM_CGB, GB_boot_rom_t_GB_BOOT_ROM_CGB_0, GB_boot_rom_t_GB_BOOT_ROM_AGB, GB_set_boot_rom_load_callback};

#[cfg(feature = "include-bootroms")]
extern fn boot_rom_load_callback(gb: *mut GB_gameboy_t, boot_rom_type: GB_boot_rom_t) {
    let boot_rom: &'static [u8] = match boot_rom_type {
        GB_boot_rom_t_GB_BOOT_ROM_DMG_0 => unimplemented!("DMG0 not yet implemented"),
        GB_boot_rom_t_GB_BOOT_ROM_DMG => include_bytes!("dmg_boot.bin"),
        GB_boot_rom_t_GB_BOOT_ROM_MGB => include_bytes!("mgb_boot.bin"),
        GB_boot_rom_t_GB_BOOT_ROM_SGB => include_bytes!("sgb_boot.bin"),
        GB_boot_rom_t_GB_BOOT_ROM_SGB2 => include_bytes!("sgb2_boot.bin"),
        GB_boot_rom_t_GB_BOOT_ROM_CGB_0 => include_bytes!("cgb0_boot.bin"),
        GB_boot_rom_t_GB_BOOT_ROM_CGB => include_bytes!("cgb_boot.bin"),
        GB_boot_rom_t_GB_BOOT_ROM_AGB => include_bytes!("agb_boot.bin"),
        _ => unreachable!("Invalid GB_boot_rom_t value")
    };

    unsafe {
        Gameboy::wrap(gb).load_boot_rom(boot_rom);
    }
}

impl Gameboy {
    #[cfg(feature = "include-bootroms")]
    pub(crate) unsafe fn init_bootrom(gb: *mut GB_gameboy_t) {
        GB_set_boot_rom_load_callback(gb, Some(boot_rom_load_callback));
    }
}

impl Gameboy {
    /// Load a boot ROM.
    pub fn load_boot_rom(&mut self, boot_rom: &[u8]) {
        unsafe {
            (*self.inner_mut()).boot_rom_unmapped = false;
            GB_load_boot_rom_from_buffer(self.as_mut_ptr(), boot_rom.as_ptr(), boot_rom.len());
        }
    }

    /// Check to see if the boot ROM has finished executing.
    pub fn boot_rom_finished(&self) -> bool {
        unsafe {
            (*self.inner()).boot_rom_unmapped
        }
    }
}
