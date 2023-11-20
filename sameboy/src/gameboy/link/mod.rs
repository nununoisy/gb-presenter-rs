pub(crate) mod printer;
mod workboy;
pub(crate) mod workboy_key;

use sameboy_sys::{GB_gameboy_t, GB_disconnect_serial, GB_serial_get_data_bit, GB_serial_set_data_bit, GB_set_infrared_callback, GB_set_infrared_input, GB_set_serial_transfer_bit_end_callback, GB_set_serial_transfer_bit_start_callback};
use super::Gameboy;

#[derive(Debug, Copy, Clone, PartialEq)]
pub(crate) enum LinkTarget {
    Console(*mut GB_gameboy_t),
    Printer,
    Workboy
}

extern fn serial_transfer_bit_start_callback(gb: *mut GB_gameboy_t, bit: bool) {
    unsafe {
        let mut gb = Gameboy::wrap(gb);
        if let Some(LinkTarget::Console(_)) = (*gb.inner_mut()).link_target {
            (*gb.inner_mut()).link_next_bit = bit;
        } else {
            panic!("Console link callback fired but link target is {:?}", (*gb.inner_mut()).link_target);
        }
    }
}

extern fn serial_transfer_bit_end_callback(gb: *mut GB_gameboy_t) -> bool {
    unsafe {
        let mut gb = Gameboy::wrap(gb);
        if let Some(LinkTarget::Console(other)) = (*gb.inner_mut()).link_target {
            let result = GB_serial_get_data_bit(other);
            GB_serial_set_data_bit(other, (*gb.inner()).link_next_bit);
            result
        } else {
            panic!("Console link callback fired but link target is {:?}", (*gb.inner_mut()).link_target);
        }
    }
}

extern fn infrared_callback(gb: *mut GB_gameboy_t, bit: bool) {
    unsafe {
        let mut gb = Gameboy::wrap(gb);
        if let Some(LinkTarget::Console(other)) = (*gb.inner_mut()).link_target {
            GB_set_infrared_input(other, bit);
        } else {
            panic!("Console IR callback fired but link target is {:?}", (*gb.inner_mut()).link_target);
        }
    }
}

impl Gameboy {
    unsafe fn disconnect_inner(&mut self) {
        (*self.inner_mut()).link_target = None;
        (*self.inner_mut()).link_next_bit = true;
        GB_disconnect_serial(self.as_mut_ptr());
        GB_set_infrared_callback(self.as_mut_ptr(), None);
    }

    unsafe fn connect_inner(&mut self, target: LinkTarget) {
        (*self.inner_mut()).link_target = Some(target);
        (*self.inner_mut()).link_next_bit = true;

        match target {
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
            (*self.inner()).link_target.is_some()
        }
    }

    pub fn disconnect(&mut self) {
        unsafe {
            match (*self.inner()).link_target {
                Some(LinkTarget::Console(gb)) => {
                    Gameboy::wrap(gb).disconnect_inner()
                },
                Some(LinkTarget::Printer) => {
                    (*self.inner_mut()).printer_receiver = None;
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
