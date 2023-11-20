use std::slice;
use sameboy_sys::{GB_gameboy_t, GB_get_pixels_output, GB_get_screen_height, GB_get_screen_width, GB_set_pixels_output, GB_set_rendering_disabled, GB_set_rgb_encode_callback, GB_set_vblank_callback, GB_vblank_type_t};
use super::Gameboy;
use super::inner::GameboyInner;

#[cfg(feature = "image")]
use image::RgbaImage;
#[cfg(feature = "image")]
use anyhow::{Result, Context};
#[cfg(feature = "image")]
use std::mem;

pub(crate) const SCREEN_BUF_SIZE: usize = 160 * 144 * 2; // reserve a bit of extra space

extern fn rgb_encode_callback_rgba(_gb: *mut GB_gameboy_t, r: u8, g: u8, b: u8) -> u32 {
    u32::from_be_bytes([r, g, b, 0xFF])
}

extern fn vblank_callback(gb: *mut GB_gameboy_t, _vblank_type: GB_vblank_type_t) {
    unsafe {
        (*Gameboy::wrap(gb).inner_mut()).vblank_occurred = true;
    }
}

impl Gameboy {
    pub(crate) unsafe fn init_video(gb: *mut GB_gameboy_t, inner: *mut GameboyInner) {
        GB_set_pixels_output(gb, (*inner).screen_buf.as_mut_ptr());
        GB_set_rgb_encode_callback(gb, Some(rgb_encode_callback_rgba));
        GB_set_vblank_callback(gb, Some(vblank_callback));
    }
}

impl Gameboy {
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
            let screen_ptr = GB_get_pixels_output(self.as_mut_ptr()) as *const u32;
            slice::from_raw_parts(screen_ptr, w * h).to_vec()
        }
    }

    #[cfg(feature = "image")]
    /// Read the current screen buffer as an RgbaImage.
    pub fn screen_image(&mut self) -> Result<RgbaImage> {
        unsafe {
            let (w, h) = self.screen_size();
            let screen_ptr = GB_get_pixels_output(self.as_mut_ptr()) as *const u32;
            let screen_buf = slice::from_raw_parts(screen_ptr as *const u8, w * h * mem::size_of::<u32>()).to_vec();

            RgbaImage::from_raw(w as _, h as _, screen_buf)
                .context("Screen image too large")
        }
    }

    /// Dump the screen to the terminal.
    pub fn dump_screen<'a>(&mut self) {
        let (w, h) = self.screen_size();
        let screen_slice: &'a [u32] = unsafe {
            let screen_ptr = GB_get_pixels_output(self.as_mut_ptr()) as *const u32;
            slice::from_raw_parts(screen_ptr, w * h)
        };

        println!("{}", "-".repeat(w / 2));
        for y in 0..(h / 2) {
            for x in 0..(w / 2) {
                let tl = ((screen_slice[(2 * x) + (2 * y) * w] >> 16) & 0xFF) < 0x90;
                let tr = ((screen_slice[(2 * x + 1) + (2 * y) * w] >> 16) & 0xFF) < 0x90;
                let bl = ((screen_slice[(2 * x) + (2 * y + 1) * w] >> 16) & 0xFF) < 0x90;
                let br = ((screen_slice[(2 * x + 1) + (2 * y + 1) * w] >> 16) & 0xFF) < 0x90;
                match (tl, tr, bl, br) {
                    (false, false, false, false) => print!(" "),
                    (false, false, false, true) =>  print!("▗"),
                    (false, false, true, false) =>  print!("▖"),
                    (false, false, true, true) =>   print!("▄"),
                    (false, true, false, false) =>  print!("▝"),
                    (false, true, false, true) =>   print!("▐"),
                    (false, true, true, false) =>   print!("▞"),
                    (false, true, true, true) =>    print!("▟"),
                    (true, false, false, false) =>  print!("▘"),
                    (true, false, false, true) =>   print!("▚"),
                    (true, false, true, false) =>   print!("▌"),
                    (true, false, true, true) =>    print!("▙"),
                    (true, true, false, false) =>   print!("▀"),
                    (true, true, false, true) =>    print!("▜"),
                    (true, true, true, false) =>    print!("▛"),
                    (true, true, true, true) =>     print!("█")
                }
            }
            println!();
        }
    }

    // TODO: stuff from display.h
}
