use std::sync::{Arc, Mutex};
use sameboy_sys::{GB_gameboy_t, GB_set_camera_get_pixel_callback, GB_set_camera_update_request_callback};
use crate::Gameboy;

extern fn camera_get_pixel_callback(gb: *mut GB_gameboy_t, x: u8, y: u8) -> u8 {
    unsafe {
        let gb = Gameboy::wrap(gb);
        let id = gb.id();

        (*gb.inner())
            .camera_provider
            .clone()
            .map(|p| p.lock().unwrap().get_pixel(id, x, y))
            .unwrap_or(0)
    }
}

extern fn camera_update_request_callback(gb: *mut GB_gameboy_t) {
    unsafe {
        let mut gb = Gameboy::wrap(gb);
        let id = gb.id();

        (*gb.inner_mut())
            .camera_provider
            .clone()
            .map(|p| p.lock().unwrap().update(id));
    }
}

pub trait CameraProvider {
    /// Called to determine the current brightness of a pixel.
    /// If using a webcam, the value of the red channel suffices.
    fn get_pixel(&self, id: usize, x: u8, y: u8) -> u8;
    /// Called whenever the image should be updated from the source.
    fn update(&mut self, id: usize);
}

impl Gameboy {
    pub(crate) unsafe fn init_camera(gb: *mut GB_gameboy_t) {
        GB_set_camera_get_pixel_callback(gb, Some(camera_get_pixel_callback));
        GB_set_camera_update_request_callback(gb, Some(camera_update_request_callback));
    }
}

impl Gameboy {
    /// Set a camera provider for cartridges that support the Game Boy Camera.
    pub fn set_camera_provider(&mut self, camera_provider: Option<Arc<Mutex<dyn CameraProvider>>>) {
        unsafe {
            (*self.inner_mut()).camera_provider = camera_provider;
        }
    }
}
