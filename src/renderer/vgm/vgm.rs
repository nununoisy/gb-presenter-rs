use std::fs;
use std::path::Path;
use super::gd3::Gd3;

#[derive(Clone, Debug)]
pub enum VgmIterItem {
    HitLoopOffset,
    WaitCommand(u16),
    DataBlock(u8, Vec<u8>),
    WriteLR35902RegCommand(u16, u8),
    InvalidCommand(u8)
}

pub struct Vgm {
    data: Vec<u8>,
    iter_ptr: usize,
    hit_loop_offset: bool,
    hit_invalid_command: bool
}

macro_rules! relative_offset_fn {
    ($name: tt, $offset: literal) => {
        pub fn $name(&self) -> usize {
            self.read_relative_offset($offset)
        }
    }
}

macro_rules! u32_fn {
    ($name: tt, $offset: literal) => {
        pub fn $name(&self) -> u32 {
            self.read_u32($offset)
        }
    }
}

impl Vgm {
    pub fn new(data: &[u8]) -> Result<Self, String> {
        if &data[0..4] != b"Vgm " {
            return Err("Invalid VGM file".to_string());
        }

        let mut result = Vgm {
            data: data.to_vec(),
            iter_ptr: 0,
            hit_loop_offset: false,
            hit_invalid_command: false
        };
        if result.version() < 0x161 {
            return Err("VGM version unsupported".to_string());
        }

        result.iter_ptr = result.start_offset();

        Ok(result)
    }

    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self, String> {
        let data = fs::read(path).map_err(|e| e.to_string())?;
        Self::new(&data)
    }

    pub fn gd3_metadata(&self) -> Option<Gd3> {
        let gd3_offset = self.read_relative_offset(0x14);
        if gd3_offset == 0x14 {
            return None;
        }

        if let Ok(gd3) = Gd3::new(&self.data[gd3_offset..]) {
            Some(gd3)
        } else {
            None
        }
    }

    fn iter_u8(&mut self) -> u8 {
        let result = self.data[self.iter_ptr];
        self.iter_ptr += 1;
        result
    }

    fn iter_u16(&mut self) -> u16 {
        u16::from_le_bytes([self.iter_u8(), self.iter_u8()])
    }

    fn iter_u32(&mut self) -> u32 {
        u32::from_le_bytes([self.iter_u8(), self.iter_u8(), self.iter_u8(), self.iter_u8()])
    }

    fn read_u32(&self, offset: usize) -> u32 {
        let mut data = [0u8; 4];
        data.copy_from_slice(&self.data[offset..offset+4]);

        u32::from_le_bytes(data)
    }

    fn read_relative_offset(&self, offset: usize) -> usize {
        (self.read_u32(offset) as usize) + offset
    }

    relative_offset_fn!(end_offset, 0x4);
    u32_fn!(version, 0x8);
    u32_fn!(sample_count, 0x18);
    relative_offset_fn!(loop_offset, 0x1C);
    u32_fn!(loop_sample_count, 0x20);
    relative_offset_fn!(start_offset, 0x34);
    u32_fn!(lr35902_clock, 0x80);
}

impl Iterator for Vgm {
    type Item = VgmIterItem;

    fn next(&mut self) -> Option<Self::Item> {
        if self.iter_ptr >= self.end_offset() || self.hit_invalid_command {
            return None;
        }
        if !self.hit_loop_offset && self.iter_ptr == self.loop_offset() {
            self.hit_loop_offset = true;
            return Some(VgmIterItem::HitLoopOffset);
        }

        let command = self.iter_u8();
        match command {
            // Wait commands
            0x61 => Some(VgmIterItem::WaitCommand(self.iter_u16())),
            0x62 => Some(VgmIterItem::WaitCommand(735)),
            0x63 => Some(VgmIterItem::WaitCommand(882)),
            0x70..=0x7F => {
                let wait = (command & 0xF) + 1;
                Some(VgmIterItem::WaitCommand(wait as u16))
            }
            // Data block
            0x67 => {
                let _compat_eof_cmd = self.iter_u8();
                let block_type = self.iter_u8();
                let block_size = self.iter_u32() as usize;

                let block = self.data[self.iter_ptr..(self.iter_ptr + block_size)].to_vec();
                self.iter_ptr += block_size;

                Some(VgmIterItem::DataBlock(block_type, block))
            }
            // LR35902 register write
            0xB3 => {
                let addr = (self.iter_u8() as u16) + 0xFF10;
                let val = self.iter_u8();
                Some(VgmIterItem::WriteLR35902RegCommand(addr, val))
            }
            // EOF
            0x66 => None,
            // Invalid
            invalid => {
                self.hit_invalid_command = true;
                Some(VgmIterItem::InvalidCommand(invalid))
            }
        }
    }
}
