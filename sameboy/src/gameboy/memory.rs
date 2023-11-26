use std::sync::{Arc, Mutex};
use std::sync::atomic::Ordering;
use super::Gameboy;
use sameboy_sys::{GB_gameboy_t, GB_set_execution_callback, GB_set_read_memory_callback, GB_set_write_memory_callback, GB_read_memory, GB_safe_read_memory, GB_write_memory, GB_set_open_bus_decay_time};
use super::inner::Dummy;

pub trait MemoryInterceptor {
    /// Intercept a memory read. Return `data` to use default behavior.
    fn intercept_read(&mut self, _id: usize, _addr: u16, data: u8) -> u8 {
        data
    }

    /// Intercept a memory write. Return `false` to block the write.
    fn intercept_write(&mut self, _id: usize, _addr: u16, _data: u8) -> bool {
        true
    }

    /// Intercept a memory execution.
    fn intercept_execute(&mut self, _id: usize, _addr: u16, _opcode: u8) {
        ()
    }
}

extern fn read_memory_callback(gb: *mut GB_gameboy_t, addr: u16, data: u8) -> u8 {
    unsafe {
        Gameboy::wrap(gb).intercept_read(addr, data)
    }
}

extern fn write_memory_callback(gb: *mut GB_gameboy_t, addr: u16, data: u8) -> bool {
    unsafe {
        Gameboy::wrap(gb).intercept_write(addr, data)
    }
}

extern fn execution_callback(gb: *mut GB_gameboy_t, addr: u16, opcode: u8) {
    unsafe {
        Gameboy::wrap(gb).intercept_execute(addr, opcode)
    }
}

impl Gameboy {
    pub(crate) unsafe fn init_memory(gb: *mut GB_gameboy_t) {
        GB_set_read_memory_callback(gb, Some(read_memory_callback));
        GB_set_write_memory_callback(gb, Some(write_memory_callback));
        GB_set_execution_callback(gb, Some(execution_callback));
    }

    #[inline(always)]
    pub(crate) unsafe fn intercept_read(&mut self, addr: u16, data: u8) -> u8 {
        (*self.inner_mut()).memory_interceptor.lock().unwrap().intercept_read(self.id(), addr, data)
    }

    #[inline(always)]
    pub(crate) unsafe fn intercept_write(&mut self, addr: u16, data: u8) -> bool {
        match addr {
            0xFF00 ..= 0xFF7F => (*(*self.inner_mut()).io_registers_copy.lock().unwrap().get_unchecked_mut((addr & 0x7F) as usize)) = data,
            _ => ()
        }

        (*self.inner_mut()).memory_interceptor.lock().unwrap().intercept_write(self.id(), addr, data)
    }

    #[inline(always)]
    pub(crate) unsafe fn intercept_execute(&mut self, addr: u16, opcode: u8) {
        match addr {
            0x0100 => (*self.inner_mut()).boot_rom_unmapped.store(true, Ordering::Release),
            _ => ()
        }

        (*self.inner_mut()).memory_interceptor.lock().unwrap().intercept_execute(self.id(), addr, opcode);
    }
}

impl Gameboy {
    /// Read a byte from memory. May trigger side-effects as a hardware read would.
    pub fn read_memory(&mut self, addr: u16) -> u8 {
        unsafe {
            GB_read_memory(self.as_mut_ptr(), addr)
        }
    }

    /// Read a byte from memory. Does not trigger side-effects.
    pub fn read_memory_safe(&mut self, addr: u16) -> u8 {
        unsafe {
            GB_safe_read_memory(self.as_mut_ptr(), addr)
        }
    }

    /// Write a byte to memory.
    pub fn write_memory(&mut self, addr: u16, data: u8) {
        unsafe {
            GB_write_memory(self.as_mut_ptr(), addr, data);
        }
    }

    /// Set a memory interceptor to hijack reads/writes/executes.
    pub fn set_memory_interceptor(&mut self, memory_interceptor: Option<Arc<Mutex<dyn MemoryInterceptor>>>) {
        unsafe {
            (*self.inner_mut()).memory_interceptor = memory_interceptor.unwrap_or(Arc::new(Mutex::new(Dummy)));
        }
    }

    /// Get a copy of the last values written to the IO registers.
    pub fn get_io_registers(&self) -> Vec<u8> {
        unsafe {
            (*self.inner()).io_registers_copy.lock().unwrap().to_vec()
        }
    }

    /// Set the amount of time required for a value to decay to $FF
    /// on the data bus, in 8MHz clocks. Set to 0 to never have
    /// values decay like, for example, an EverDrive.
    pub fn set_open_bus_decay_time(&mut self, decay_time: u32) {
        unsafe {
            GB_set_open_bus_decay_time(self.as_mut_ptr(), decay_time);
        }
    }
}
