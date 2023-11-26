pub(crate) mod printer;
mod workboy;
pub(crate) mod workboy_key;

use std::sync::{Arc, Mutex};
use std::sync::atomic::Ordering;
use sameboy_sys::{GB_gameboy_t, GB_disconnect_serial, GB_serial_get_data_bit, GB_serial_set_data_bit, GB_set_infrared_callback, GB_set_infrared_input, GB_set_serial_transfer_bit_end_callback, GB_set_serial_transfer_bit_start_callback};
use super::Gameboy;
use super::inner::Dummy;

#[derive(Debug, Copy, Clone, PartialEq)]
pub(crate) enum LinkTarget {
    None,
    Console(*mut GB_gameboy_t),
    Printer,
    Workboy
}

extern fn serial_transfer_bit_start_callback(gb: *mut GB_gameboy_t, bit: bool) {
    unsafe {
        let mut gb = Gameboy::wrap(gb);
        if let LinkTarget::Console(_) = *(*gb.inner_mut()).link_target.lock().unwrap() {
            (*gb.inner_mut()).link_next_bit.store(bit, Ordering::SeqCst);
        } else {
            panic!("Console link callback fired but link target is {:?}", (*gb.inner_mut()).link_target.lock().unwrap());
        }
    }
}

extern fn serial_transfer_bit_end_callback(gb: *mut GB_gameboy_t) -> bool {
    unsafe {
        let mut gb = Gameboy::wrap(gb);
        if let LinkTarget::Console(other) = *(*gb.inner_mut()).link_target.lock().unwrap() {
            let result = GB_serial_get_data_bit(other);
            GB_serial_set_data_bit(other, (*gb.inner()).link_next_bit.load(Ordering::SeqCst));
            result
        } else {
            panic!("Console link callback fired but link target is {:?}", (*gb.inner_mut()).link_target.lock().unwrap());
        }
    }
}

extern fn infrared_callback(gb: *mut GB_gameboy_t, bit: bool) {
    unsafe {
        let mut gb = Gameboy::wrap(gb);
        if let LinkTarget::Console(other) = *(*gb.inner_mut()).link_target.lock().unwrap() {
            GB_set_infrared_input(other, bit);
        } else {
            panic!("Console IR callback fired but link target is {:?}", (*gb.inner_mut()).link_target.lock().unwrap());
        }
    }
}

impl Gameboy {
    unsafe fn disconnect_inner(&mut self) {
        (*(*self.inner_mut()).link_target.lock().unwrap()) = LinkTarget::None;
        (*self.inner_mut()).link_next_bit.store(true, Ordering::SeqCst);
        GB_disconnect_serial(self.as_mut_ptr());
        GB_set_infrared_callback(self.as_mut_ptr(), None);
    }

    unsafe fn connect_inner(&mut self, target: LinkTarget) {
        (*(*self.inner_mut()).link_target.lock().unwrap()) = target;
        (*self.inner_mut()).link_next_bit.store(true, Ordering::SeqCst);

        match target {
            LinkTarget::None => (),
            LinkTarget::Console(_) => {
                GB_set_serial_transfer_bit_start_callback(self.as_mut_ptr(), Some(serial_transfer_bit_start_callback));
                GB_set_serial_transfer_bit_end_callback(self.as_mut_ptr(), Some(serial_transfer_bit_end_callback));
                GB_set_infrared_callback(self.as_mut_ptr(), Some(infrared_callback));
            },
            LinkTarget::Printer => self.connect_printer_inner(),
            LinkTarget::Workboy => self.connect_workboy_inner()
        }
    }
}

impl Gameboy {
    pub fn connected(&self) -> bool {
        unsafe {
            (*(*self.inner()).link_target.lock().unwrap()) != LinkTarget::None
        }
    }

    pub fn disconnect(&mut self) {
        unsafe {
            match *(*self.inner()).link_target.lock().unwrap() {
                LinkTarget::Console(gb) => {
                    Gameboy::wrap(gb).disconnect_inner()
                },
                LinkTarget::Printer => {
                    (*self.inner_mut()).printer_receiver = Arc::new(Mutex::new(Dummy));
                }
                _ => ()
            }
            self.disconnect_inner();
        }
    }

    pub fn connect_console(&mut self, other: &mut Gameboy) {
        unsafe {
            self.disconnect();
            other.disconnect();

            self.connect_inner(LinkTarget::Console(other.as_mut_ptr()));
            other.connect_inner(LinkTarget::Console(self.as_mut_ptr()));
        }
    }


}
