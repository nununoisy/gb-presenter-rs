use super::Gameboy;
use sameboy_sys::{GB_direct_access_t_GB_DIRECT_ACCESS_IO, GB_gameboy_t, GB_safe_read_memory, GB_set_execution_callback, GB_set_read_memory_callback, GB_set_write_memory_callback, GB_write_memory};

pub trait MemoryInterceptor {
    /// Intercept a memory read. Return `data` to use default behavior.
    fn intercept_read(&mut self, addr: u16, data: u8) -> u8 {
        data
    }

    /// Intercept a memory write. Return `false` to block the default write.
    fn intercept_write(&mut self, addr: u16, data: u8) -> bool {
        true
    }

    /// Intercept a memory execution.
    fn intercept_execute(&mut self, addr: u16, opcode: u8) {
        ()
    }
}

extern fn read_memory_callback(gb: *mut GB_gameboy_t, addr: u16, data: u8) -> u8 {
    unsafe {
        let gb = Gameboy::mut_from_callback_ptr(gb);
        if let Some(interceptor) = &mut gb.memory_interceptor {
            interceptor.intercept_read(addr, data)
        } else {
            data
        }
    }
}

extern fn write_memory_callback(gb: *mut GB_gameboy_t, addr: u16, data: u8) -> bool {
    unsafe {
        let gb = Gameboy::mut_from_callback_ptr(gb);

        match addr {
            0xFF00..=0xFF7F => gb.io_registers_copy[(addr & 0xFF) as usize] = data,
            _ => ()
        }

        if let Some(interceptor) = &mut gb.memory_interceptor {
            interceptor.intercept_write(addr, data)
        } else {
            true
        }
    }
}

extern fn execution_callback(gb: *mut GB_gameboy_t, addr: u16, opcode: u8) {
    unsafe {
        let gb = Gameboy::mut_from_callback_ptr(gb);

        match addr {
            0x0100 => gb.boot_rom_unmapped = true,
            _ => ()
        }

        if let Some(interceptor) = &mut gb.memory_interceptor {
            interceptor.intercept_execute(addr, opcode);
        }
    }
}

impl Gameboy {
    /// Read a byte from memory.
    pub fn read_memory(&mut self, addr: u16) -> u8 {
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

    pub fn set_memory_interceptor(&mut self, memory_interceptor: Option<Box<dyn MemoryInterceptor>>) {
        self.memory_interceptor = memory_interceptor;
        unsafe {
            GB_set_read_memory_callback(self.as_mut_ptr(), Some(read_memory_callback));
            GB_set_write_memory_callback(self.as_mut_ptr(), Some(write_memory_callback));
            GB_set_execution_callback(self.as_mut_ptr(), Some(execution_callback));
        }
    }

    pub fn get_io_registers(&mut self) -> [u8; 0x80] {
        unsafe {
            let mut result = [0u8; 0x80];

            // let (io_registers, _bank) = self.direct_access(GB_direct_access_t_GB_DIRECT_ACCESS_IO);
            result.copy_from_slice(&self.io_registers_copy);

            // Do some extra reads to fill in dynamic registers
            // for i in 0x10..=0x2F {
            //     // APU registers
            //     result[i] = self.read_memory(0xFF00 | (i as u16));
            // }
            // result[0x76] = self.read_memory(0xFF76);  // PCM12
            // result[0x77] = self.read_memory(0xFF77);  // PCM34

            result
        }
    }
}
