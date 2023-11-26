use std::{ptr, slice};
use std::mem;
use std::sync::atomic::Ordering;
use sameboy_sys::{GB_color_correction_mode_t, GB_color_correction_mode_t_GB_COLOR_CORRECTION_CORRECT_CURVES, GB_color_correction_mode_t_GB_COLOR_CORRECTION_DISABLED, GB_color_correction_mode_t_GB_COLOR_CORRECTION_LOW_CONTRAST, GB_color_correction_mode_t_GB_COLOR_CORRECTION_MODERN_ACCURATE, GB_color_correction_mode_t_GB_COLOR_CORRECTION_MODERN_BALANCED, GB_color_correction_mode_t_GB_COLOR_CORRECTION_MODERN_BOOST_CONTRAST, GB_color_correction_mode_t_GB_COLOR_CORRECTION_REDUCE_CONTRAST, GB_convert_rgb15, GB_draw_tilemap, GB_draw_tileset, GB_gameboy_t, GB_get_oam_info, GB_get_pixels_output, GB_get_screen_height, GB_get_screen_width, GB_is_background_rendering_disabled, GB_is_object_rendering_disabled, GB_is_odd_frame, GB_map_type_t, GB_map_type_t_GB_MAP_9800, GB_map_type_t_GB_MAP_9C00, GB_map_type_t_GB_MAP_AUTO, GB_oam_info_t, GB_palette_type_t, GB_palette_type_t_GB_PALETTE_AUTO, GB_palette_type_t_GB_PALETTE_BACKGROUND, GB_palette_type_t_GB_PALETTE_NONE, GB_palette_type_t_GB_PALETTE_OAM, GB_set_background_rendering_disabled, GB_set_color_correction_mode, GB_set_light_temperature, GB_set_object_rendering_disabled, GB_set_pixels_output, GB_set_rendering_disabled, GB_set_rgb_encode_callback, GB_set_vblank_callback, GB_tileset_type_t, GB_tileset_type_t_GB_TILESET_8000, GB_tileset_type_t_GB_TILESET_8800, GB_tileset_type_t_GB_TILESET_AUTO, GB_vblank_type_t};
use super::Gameboy;

#[cfg(feature = "image")]
use image::RgbaImage;
#[cfg(feature = "image")]
use anyhow::{Result, Context};

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum PaletteType {
    None,
    Background,
    OAM,
    Auto
}

impl From<GB_palette_type_t> for PaletteType {
    fn from(value: GB_palette_type_t) -> Self {
        match value {
            GB_palette_type_t_GB_PALETTE_NONE => Self::None,
            GB_palette_type_t_GB_PALETTE_BACKGROUND => Self::Background,
            GB_palette_type_t_GB_PALETTE_OAM => Self::OAM,
            GB_palette_type_t_GB_PALETTE_AUTO => Self::Auto,
            _ => unreachable!("Invalid GB_palette_type_t value")
        }
    }
}

impl From<PaletteType> for GB_palette_type_t {
    fn from(value: PaletteType) -> Self {
        match value {
            PaletteType::None => GB_palette_type_t_GB_PALETTE_NONE,
            PaletteType::Background => GB_palette_type_t_GB_PALETTE_BACKGROUND,
            PaletteType::OAM => GB_palette_type_t_GB_PALETTE_OAM,
            PaletteType::Auto => GB_palette_type_t_GB_PALETTE_AUTO
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum MapType {
    Auto,
    Map9800,
    Map9C00
}

impl From<GB_map_type_t> for MapType {
    fn from(value: GB_map_type_t) -> Self {
        match value {
            GB_map_type_t_GB_MAP_AUTO => Self::Auto,
            GB_map_type_t_GB_MAP_9800 => Self::Map9800,
            GB_map_type_t_GB_MAP_9C00 => Self::Map9C00,
            _ => unreachable!("Invalid GB_map_type_t value")
        }
    }
}

impl From<MapType> for GB_map_type_t {
    fn from(value: MapType) -> Self {
        match value {
            MapType::Auto => GB_map_type_t_GB_MAP_AUTO,
            MapType::Map9800 => GB_map_type_t_GB_MAP_9800,
            MapType::Map9C00 => GB_map_type_t_GB_MAP_9C00
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum TilesetType {
    Auto,
    Tileset8000,
    Tileset8800
}

impl From<GB_tileset_type_t> for TilesetType {
    fn from(value: GB_tileset_type_t) -> Self {
        match value {
            GB_tileset_type_t_GB_TILESET_AUTO => Self::Auto,
            GB_tileset_type_t_GB_TILESET_8000 => Self::Tileset8000,
            GB_tileset_type_t_GB_TILESET_8800 => Self::Tileset8800,
            _ => unreachable!("Invalid GB_tileset_type_t value")
        }
    }
}

impl From<TilesetType> for GB_tileset_type_t {
    fn from(value: TilesetType) -> Self {
        match value {
            TilesetType::Auto => GB_tileset_type_t_GB_TILESET_AUTO,
            TilesetType::Tileset8000 => GB_tileset_type_t_GB_TILESET_8000,
            TilesetType::Tileset8800 => GB_tileset_type_t_GB_TILESET_8800
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum ColorCorrectionMode {
    Disabled,
    CorrectCurves,
    Balanced,
    BoostContrast,
    ReduceContrast,
    LowContrast,
    Accurate
}

impl From<GB_color_correction_mode_t> for ColorCorrectionMode {
    fn from(value: GB_color_correction_mode_t) -> Self {
        match value {
            GB_color_correction_mode_t_GB_COLOR_CORRECTION_DISABLED => Self::Disabled,
            GB_color_correction_mode_t_GB_COLOR_CORRECTION_CORRECT_CURVES => Self::CorrectCurves,
            GB_color_correction_mode_t_GB_COLOR_CORRECTION_MODERN_BALANCED => Self::Balanced,
            GB_color_correction_mode_t_GB_COLOR_CORRECTION_MODERN_BOOST_CONTRAST => Self::BoostContrast,
            GB_color_correction_mode_t_GB_COLOR_CORRECTION_REDUCE_CONTRAST => Self::ReduceContrast,
            GB_color_correction_mode_t_GB_COLOR_CORRECTION_LOW_CONTRAST => Self::LowContrast,
            GB_color_correction_mode_t_GB_COLOR_CORRECTION_MODERN_ACCURATE => Self::Accurate,
            _ => unreachable!("Invalid GB_color_correction_mode_t value")
        }
    }
}

impl From<ColorCorrectionMode> for GB_color_correction_mode_t {
    fn from(value: ColorCorrectionMode) -> Self {
        match value {
            ColorCorrectionMode::Disabled => GB_color_correction_mode_t_GB_COLOR_CORRECTION_DISABLED,
            ColorCorrectionMode::CorrectCurves => GB_color_correction_mode_t_GB_COLOR_CORRECTION_CORRECT_CURVES,
            ColorCorrectionMode::Balanced => GB_color_correction_mode_t_GB_COLOR_CORRECTION_MODERN_BALANCED,
            ColorCorrectionMode::BoostContrast => GB_color_correction_mode_t_GB_COLOR_CORRECTION_MODERN_BOOST_CONTRAST,
            ColorCorrectionMode::ReduceContrast => GB_color_correction_mode_t_GB_COLOR_CORRECTION_REDUCE_CONTRAST,
            ColorCorrectionMode::LowContrast => GB_color_correction_mode_t_GB_COLOR_CORRECTION_LOW_CONTRAST,
            ColorCorrectionMode::Accurate => GB_color_correction_mode_t_GB_COLOR_CORRECTION_MODERN_ACCURATE
        }
    }
}

#[derive(Copy, Clone)]
#[repr(transparent)]
pub struct OamInfo(GB_oam_info_t);

macro_rules! oam_flag_fn {
    ($name: ident, $mask: literal) => {
        pub fn $name(&self) -> bool {
            (self.0.flags & $mask) != 0
        }
    };
}

impl OamInfo {
    pub fn position(&self) -> (u8, u8) {
        (self.0.x, self.0.y)
    }

    oam_flag_fn!(priority, 0b1000_0000);
    oam_flag_fn!(vertical_mirroring, 0b0100_0000);
    oam_flag_fn!(horizontal_mirroring, 0b0010_0000);
    oam_flag_fn!(dmg_use_obp1, 0b0001_0000);
    oam_flag_fn!(cgb_use_vram_bank_1, 0b0000_1000);

    pub fn cgb_palette_index(&self) -> u8 {
        self.0.flags & 0b0000_0111
    }

    pub fn tile_index(&self) -> u8 {
        self.0.tile
    }

    pub fn address(&self) -> u16 {
        self.0.oam_addr
    }

    pub fn is_obscured_by_line_limit(&self) -> bool {
        self.0.obscured_by_line_limit
    }

    pub fn image_data(&self, height: u8) -> Vec<u32> {
        self.0.image[0..(8 * height as usize)].to_vec()
    }

    #[cfg(feature = "image")]
    pub fn image(&self, height: u8) -> Result<RgbaImage> {
        let image_data = unsafe { slice::from_raw_parts(self.0.image.as_ptr() as *const u8, 8 * height as usize * mem::size_of::<u32>()).to_vec() };

        RgbaImage::from_raw(8, height as u32, image_data)
            .context("OAM image too big")
    }
}

pub(crate) const SCREEN_BUF_SIZE: usize = 160 * 144 * 2; // reserve a bit of extra space

extern fn rgb_encode_callback(_gb: *mut GB_gameboy_t, r: u8, g: u8, b: u8) -> u32 {
    (((r as u32) << 24) | ((g as u32) << 16) | ((b as u32) << 8) | 0xFF).to_be()
}

extern fn vblank_callback(gb: *mut GB_gameboy_t, _vblank_type: GB_vblank_type_t) {
    unsafe {
        (*Gameboy::wrap(gb).inner_mut()).vblank_occurred.store(true, Ordering::SeqCst);
    }
}

impl Gameboy {
    pub(crate) unsafe fn init_video(gb: *mut GB_gameboy_t, screen_buf_ptr: *mut u32) {
        GB_set_pixels_output(gb, screen_buf_ptr);
        GB_set_rendering_disabled(gb, false);
        GB_set_rgb_encode_callback(gb, Some(rgb_encode_callback));
        GB_set_vblank_callback(gb, Some(vblank_callback));
    }
}

impl Gameboy {
    /// Enable or disable rendering.
    pub fn set_rendering_disabled(&mut self, rendering_disabled: bool) {
        unsafe {
            (*self.inner_mut()).rendering_disabled.store(rendering_disabled, Ordering::SeqCst);
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

    unsafe fn screen_data<T: Sized>(&mut self) -> &[T] {
        let (w, h) = self.screen_size();

        // Prevent SameBoy from writing to the screen buffer while we access it.
        GB_set_rendering_disabled(self.as_mut_ptr(), true);

        let screen_ptr = GB_get_pixels_output(self.as_mut_ptr()) as *mut T;
        let result = slice::from_raw_parts(screen_ptr, w * h * mem::size_of::<T>() / mem::size_of::<u32>());

        GB_set_rendering_disabled(self.as_mut_ptr(), (*self.inner()).rendering_disabled.load(Ordering::SeqCst));

        result
    }

    /// Read the current screen buffer as a vector of 32-bit RGBA values.
    pub fn screen_buffer(&mut self) -> Vec<u32> {
        unsafe {
            self.screen_data::<u32>().to_vec()
        }
    }

    /// Read the current screen buffer as an RgbaImage.
    #[cfg(feature = "image")]
    pub fn screen_image(&mut self) -> Result<RgbaImage> {
        let (w, h) = self.screen_size();
        // This is safe because the screen data is always stored as big-endian RGBA.
        let screen_buf = unsafe { self.screen_data::<u8>().to_vec() };

        RgbaImage::from_raw(w as _, h as _, screen_buf)
            .context("Screen image too large")
    }

    /// Dump the screen to the terminal.
    pub fn dump_screen(&mut self) {
        let (w, h) = self.screen_size();
        println!("{}", "-".repeat(w));

        unsafe {
            let screen_buf = self.screen_data::<u32>();

            for y in 0..h {
                for x in 0..w {
                    let pixel = screen_buf[x + y * w].to_ne_bytes();
                    print!("\x1B[38;2;{};{};{}m█", pixel[0], pixel[1], pixel[2]);
                }
                println!("\x1B[0m");
            }
        }
    }

    /// Draw the current tileset data to a raw RGBA pixel array. The size is always 256x192.
    pub fn draw_tileset(&mut self, palette_type: PaletteType, palette_index: u8) -> [u32; 256 * 192] {
        let mut result = [0u32; 256 * 192];
        unsafe {
            GB_draw_tileset(self.as_mut_ptr(), result.as_mut_ptr(), palette_type.into(), palette_index);
        }
        result
    }

    /// Draw the current tileset data to an RgbaImage. The size is always 256x192.
    #[cfg(feature = "image")]
    pub fn tileset_image(&mut self, palette_type: PaletteType, palette_index: u8) -> Result<RgbaImage> {
        let tileset_buf = self.draw_tileset(palette_type, palette_index);
        let image_buf: Vec<u8> = unsafe {
            slice::from_raw_parts(tileset_buf.as_ptr() as *const u8, tileset_buf.len() * mem::size_of::<u32>()).to_vec()
        };
        RgbaImage::from_raw(256, 192, image_buf)
            .context("Tileset image too large")
    }

    /// Draw the current tilemap data to a raw RGBA pixel array. The size is always 256x256.
    pub fn draw_tilemap(&mut self, palette_type: PaletteType, palette_index: u8, map_type: MapType, tileset_type: TilesetType) -> [u32; 256 * 256] {
        let mut result = [0u32; 256 * 256];
        unsafe {
            GB_draw_tilemap(self.as_mut_ptr(), result.as_mut_ptr(), palette_type.into(), palette_index, map_type.into(), tileset_type.into());
        }
        result
    }

    /// Draw the current tilemap data to an RgbaImage. The size is always 256x256.
    #[cfg(feature = "image")]
    pub fn tilemap_image(&mut self, palette_type: PaletteType, palette_index: u8, map_type: MapType, tileset_type: TilesetType) -> Result<RgbaImage> {
        let tilemap_buf = self.draw_tilemap(palette_type, palette_index, map_type, tileset_type);
        let image_buf: Vec<u8> = unsafe {
            slice::from_raw_parts(tilemap_buf.as_ptr() as *const u8, tilemap_buf.len() * mem::size_of::<u32>()).to_vec()
        };
        RgbaImage::from_raw(256, 256, image_buf)
            .context("Tilemap image too large")
    }

    /// Get information about objects in OAM.
    pub fn oam_info(&mut self) -> (Vec<OamInfo>, u8) {
        unsafe {
            let mut object_info: [mem::MaybeUninit<OamInfo>; 40] = mem::MaybeUninit::uninit().assume_init();
            let mut object_height: u8 = 0;
            let object_count = GB_get_oam_info(self.as_mut_ptr(), object_info.as_mut_ptr() as *mut GB_oam_info_t, ptr::addr_of_mut!(object_height)) as usize;

            let result: Vec<OamInfo> = object_info
                .iter()
                .take(object_count)
                .cloned()
                .map(|o| o.assume_init())
                .collect();

            for o in &mut object_info {
                o.assume_init_drop();
            }

            (result, object_height)
        }
    }

    pub fn convert_rgb15(&mut self, color: u16, for_border: bool) -> u32 {
        unsafe {
            GB_convert_rgb15(self.as_mut_ptr(), color, for_border)
        }
    }

    pub fn set_color_correction_mode(&mut self, color_correction_mode: ColorCorrectionMode) {
        unsafe {
            GB_set_color_correction_mode(self.as_mut_ptr(), color_correction_mode.into());
        }
    }

    pub fn set_light_temperature(&mut self, temperature: f64) {
        unsafe {
            GB_set_light_temperature(self.as_mut_ptr(), temperature);
        }
    }

    pub fn current_frame_is_odd(&mut self) -> bool {
        unsafe {
            GB_is_odd_frame(self.as_mut_ptr())
        }
    }

    pub fn object_rendering_disabled(&mut self) -> bool {
        unsafe {
            GB_is_object_rendering_disabled(self.as_mut_ptr())
        }
    }

    pub fn set_object_rendering_disabled(&mut self, disabled: bool) {
        unsafe {
            GB_set_object_rendering_disabled(self.as_mut_ptr(), disabled);
        }
    }

    pub fn background_rendering_disabled(&mut self) -> bool {
        unsafe {
            GB_is_background_rendering_disabled(self.as_mut_ptr())
        }
    }

    pub fn set_background_rendering_disabled(&mut self, disabled: bool) {
        unsafe {
            GB_set_background_rendering_disabled(self.as_mut_ptr(), disabled);
        }
    }
}
