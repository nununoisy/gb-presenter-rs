mod joypad;
mod model;
mod audio;
mod memory;

use core::slice;
use std::cell::RefCell;
use std::collections::VecDeque;
use std::ffi::CString;
use std::ptr;
use std::rc::Rc;
use sameboy_sys::{GB_alloc, GB_dealloc, GB_direct_access_t, GB_gameboy_t, GB_gbs_switch_track, GB_get_direct_access, GB_get_pixels_output, GB_get_rom_title, GB_get_screen_height, GB_get_screen_width, GB_get_user_data, GB_init, GB_load_battery_from_buffer, GB_load_boot_rom_from_buffer, GB_load_gbs_from_buffer, GB_load_rom_from_buffer, GB_reset, GB_run, GB_run_frame, GB_set_pixels_output, GB_set_rendering_disabled, GB_set_rgb_encode_callback, GB_set_user_data};

pub use joypad::JoypadButton;
pub use audio::{ApuChannel, ApuStateReceiver};
pub use model::{Model, Revision, VideoStandard};
pub use memory::MemoryInterceptor;

const SCREEN_BUF_SIZE: usize = 160 * 144 * 2; // reserve a bit of extra space

extern fn rgb_encode_callback(_gb: *mut GB_gameboy_t, r: u8, g: u8, b: u8) -> u32 {
    ((r as u32) << 24) | ((g as u32) << 16) | ((b as u32) << 8) | 0xFF
}

#[repr(C)]
pub struct Gameboy {
    ptr: *mut GB_gameboy_t,
    audio_buf: VecDeque<i16>,
    memory_interceptor: Option<Box<dyn MemoryInterceptor>>,
    io_registers_copy: [u8; 0x80],
    apu_receiver: Option<Rc<RefCell<dyn ApuStateReceiver>>>,
    screen_buf: [u32; SCREEN_BUF_SIZE]
}

impl Gameboy {
    pub unsafe fn as_ptr(&self) -> *const GB_gameboy_t {
        self.ptr as *const _
    }

    pub unsafe fn as_mut_ptr(&mut self) -> *mut GB_gameboy_t {
        self.ptr
    }

    unsafe fn direct_access(&mut self, access_type: GB_direct_access_t) -> (Vec<u8>, u16) {
        let mut size = 0usize;
        let mut bank = 0u16;

        let da_ptr = GB_get_direct_access(self.as_mut_ptr(), access_type, ptr::addr_of_mut!(size), ptr::addr_of_mut!(bank)) as *const u8;
        let mut result = vec![0u8; size];
        result.copy_from_slice(slice::from_raw_parts(da_ptr, size));

        (result, bank)
    }

    pub unsafe fn mut_from_callback_ptr<'a>(gb: *mut GB_gameboy_t) -> &'a mut Self {
        (GB_get_user_data(gb) as *mut Gameboy).as_mut().unwrap()
    }
}

impl Gameboy {
    // TODO: this is a self-referential struct without a Pin. Fix that.
    pub fn new(model: Model) -> Box<Self> {
        unsafe {
            let mut result = Box::new(Self {
                ptr: ptr::null_mut(),
                screen_buf: [0u32; SCREEN_BUF_SIZE],
                audio_buf: VecDeque::new(),
                memory_interceptor: None,
                io_registers_copy: [0u8; 0x80],
                apu_receiver: None
            });

            result.ptr = GB_alloc();
            GB_init(result.as_mut_ptr(), model.into());
            GB_set_user_data(result.as_mut_ptr(), &mut *result as *mut Gameboy as *mut _);
            GB_set_pixels_output(result.as_mut_ptr(), result.screen_buf.as_mut_ptr());
            GB_set_rgb_encode_callback(result.as_mut_ptr(), Some(rgb_encode_callback));
            result.init_audio();
            result.set_memory_interceptor(None);

            result
        }
    }

    /// Reset the emulation.
    pub fn reset(&mut self) {
        unsafe {
            GB_reset(self.as_mut_ptr());
        }
    }

    /// Load a boot ROM.
    pub fn load_boot_rom(&mut self, boot_rom: &[u8]) {
        unsafe {
            GB_load_boot_rom_from_buffer(self.as_mut_ptr(), boot_rom.as_ptr(), boot_rom.len());
        }
    }

    /// Load a cartridge ROM.
    pub fn load_rom(&mut self, rom: &[u8]) {
        unsafe {
            GB_load_rom_from_buffer(self.as_mut_ptr(), rom.as_ptr(), rom.len());
        }
    }

    /// Load a cartridge battery save.
    pub fn load_sram(&mut self, sram: &[u8]) {
        unsafe {
            GB_load_battery_from_buffer(self.as_mut_ptr(), sram.as_ptr(), sram.len());
        }
    }

    /// Load a GameBoy Sound module.
    pub fn load_gbs(&mut self, gbs: &[u8]) {
        unsafe {
            // todo info
            GB_load_gbs_from_buffer(self.as_mut_ptr(), gbs.as_ptr(), gbs.len(), ptr::null_mut());
        }
    }

    pub fn gbs_change_track(&mut self, track: u8) {
        unsafe {
            GB_gbs_switch_track(self.as_mut_ptr(), track);
        }
    }

    pub fn game_title(&mut self) -> String {
        unsafe {
            let mut title_bytes = vec![0u8; 16];
            GB_get_rom_title(self.as_mut_ptr(), title_bytes.as_mut_ptr() as *mut _);

            if title_bytes.iter().all(|&b| b == 0) {
                return "".to_string();
            }

            if let Some(terminator) = title_bytes.iter().rposition(|&b| b != 0) {
                title_bytes.truncate(terminator + 1);
            }
            title_bytes.push(0);

            CString::from_vec_with_nul(title_bytes).unwrap().into_string().unwrap()
        }
    }

    /// Run for a single clock cycle. Returns the number of 8MHz ticks passed.
    pub fn run(&mut self) -> usize {
        unsafe {
            GB_run(self.as_mut_ptr()) as usize
        }
    }

    /// Run for a single frame. Returns the time passed in nanoseconds.
    pub fn run_frame(&mut self) -> u64 {
        unsafe {
            GB_run_frame(self.as_mut_ptr())
        }
    }

    /// Enable or disable rendering.
    pub fn set_rendering_disabled(&mut self, rendering_disabled: bool) {
        unsafe {
            GB_set_rendering_disabled(self.as_mut_ptr(), rendering_disabled);
        }
    }

    /// Get the size of the emulated screen.
    pub fn screen_size(&mut self) -> (usize, usize) {
        unsafe {
            let w = GB_get_screen_width(self.as_mut_ptr()) as usize;
            let h = GB_get_screen_height(self.as_mut_ptr()) as usize;
            (w, h)
        }
    }

    /// Read the current screen buffer as a vector of 32-bit RGBA values.
    pub fn screen_buffer(&mut self) -> Vec<u32> {
        unsafe {
            let (w, h) = self.screen_size();
            let mut result = vec![0u32; w * h];
            let screen_ptr = GB_get_pixels_output(self.as_mut_ptr()) as *const u32;
            result.copy_from_slice(slice::from_raw_parts(screen_ptr, w * h));
            result
        }
    }

    pub fn screen_buffer_rgb(&mut self) -> Vec<u8> {
        let (w, h) = self.screen_size();
        let mut result = vec![0u8; 3 * w * h];

        for (i, pixel) in self.screen_buffer().iter().enumerate() {
            let r = ((*pixel >> 24) & 0xFF) as u8;
            let g = ((*pixel >> 16) & 0xFF) as u8;
            let b = ((*pixel >> 8) & 0xFF) as u8;

            result[3*i] = r;
            result[3*i+1] = g;
            result[3*i+2] = b;
        }

        result
    }
}

impl Drop for Gameboy {
    fn drop(&mut self) {
        unsafe {
            GB_dealloc(self.as_mut_ptr());
        }
    }
}