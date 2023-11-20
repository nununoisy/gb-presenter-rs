use sameboy_sys::{GB_connect_printer, GB_gameboy_t};
use std::slice;
use std::sync::{Arc, Mutex};
use super::LinkTarget;
use super::super::Gameboy;

extern fn print_image_callback(gb: *mut GB_gameboy_t, image: *mut u32, height: u8, top_margin: u8, bottom_margin: u8, exposure: u8) {
    unsafe {
        let mut gb = Gameboy::wrap(gb);
        let id = gb.id();

        let image = slice::from_raw_parts(image.cast_const(), 160 * height as usize).to_vec();

        (*gb.inner_mut())
            .printer_receiver
            .clone()
            .map(|r| r.lock().unwrap().print_data_updated(id, &image, top_margin, bottom_margin, exposure));
    }
}

extern fn printer_done_callback(gb: *mut GB_gameboy_t) {
    unsafe {
        let mut gb = Gameboy::wrap(gb);
        let id = gb.id();

        (*gb.inner_mut())
            .printer_receiver
            .clone()
            .map(|r| r.lock().unwrap().print_finished(id));
    }
}

pub trait PrinterReceiver {
    /// Called whenever the printed image data changes.
    /// The image is always 160 pixels wide.
    fn print_data_updated(&mut self, id: usize, image: &[u32], top_margin: u8, bottom_margin: u8, exposure: u8);
    /// Called when the printer finishes printing the current image.
    fn print_finished(&mut self, id: usize);
}

impl Gameboy {
    pub(crate) unsafe fn connect_printer_inner(&mut self) {
        GB_connect_printer(self.as_mut_ptr(), Some(print_image_callback), Some(printer_done_callback));
    }
}

impl Gameboy {
    pub fn connect_printer(&mut self, printer_reciever: Arc<Mutex<dyn PrinterReceiver>>) {
        unsafe {
            (*self.inner_mut()).printer_receiver = Some(printer_reciever);

            self.disconnect();
            self.connect_inner(LinkTarget::Printer);
        }
    }
}
