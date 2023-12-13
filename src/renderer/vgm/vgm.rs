use anyhow::{Result, ensure, Context};
use std::fs;
use std::io::Read;
use std::path::Path;
use super::gd3::Gd3;
use flate2::read::GzDecoder;

#[derive(Clone, Debug)]
pub enum VgmIterItem {
    HitLoopOffset,
    WaitCommand(u16),
    DataBlock(u8, Vec<u8>),
    WriteLR35902RegCommand(bool, u16, u8),
    InvalidCommand(u8)
}

pub struct Vgm {
    data: Vec<u8>
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

const CLOCK_FLAG_IS_2X: u32 = 0x40000000;

macro_rules! clock_fn {
    ($name: tt, $offset: literal) => {
        pub fn $name(&self) -> Option<(u32, bool)> {
            match self.read_u32($offset) {
                0 => None,
                value => Some((value & !CLOCK_FLAG_IS_2X, (value & CLOCK_FLAG_IS_2X) != 0))
            }
        }
    }
}

impl Vgm {
    pub fn new(data: &[u8]) -> Result<Self> {
        let mut result = Self { data: Vec::new() };
        if &data[0..4] == b"Vgm " {
            result.data.extend_from_slice(data);
        } else {
            let mut decoder = GzDecoder::new(data);
            ensure!(decoder.header().is_some(), "Input data is not a valid VGM or VGZ");
            decoder.read_to_end(&mut result.data).context("VGZ inflate failed")?;
            ensure!(&result.data[0..4] == b"Vgm ", "Input data is a valid gzip file but not a VGZ");
        };
        ensure!(result.version() >= 0x161, "VGM version not supported");
        ensure!(result.lr35902_clock().is_some(), "VGM does not contain a Game Boy");
        Ok(result)
    }

    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self> {
        let data = fs::read(path)?;
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

    pub(super) fn read_u8(&self, offset: usize) -> u8 {
        self.data[offset]
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
    clock_fn!(lr35902_clock, 0x80);

    pub fn iter(&self) -> VgmIterator<'_> {
        VgmIterator::new(&self)
    }
}

pub struct VgmIterator<'a> {
    vgm: &'a Vgm,
    iter_ptr: usize,
    hit_loop_offset: bool,
    hit_invalid_command: bool
}

impl<'a> VgmIterator<'a> {
    pub(super) fn new(vgm: &'a Vgm) -> Self {
        Self {
            vgm,
            iter_ptr: vgm.start_offset(),
            hit_loop_offset: false,
            hit_invalid_command: false
        }
    }

    fn iter_u8(&mut self) -> u8 {
        let result = self.vgm.read_u8(self.iter_ptr);
        self.iter_ptr += 1;
        result
    }

    fn iter_u16(&mut self) -> u16 {
        u16::from_le_bytes([self.iter_u8(), self.iter_u8()])
    }

    fn iter_u32(&mut self) -> u32 {
        u32::from_le_bytes([self.iter_u8(), self.iter_u8(), self.iter_u8(), self.iter_u8()])
    }
}

impl Iterator for VgmIterator<'_> {
    type Item = VgmIterItem;

    fn next(&mut self) -> Option<Self::Item> {
        if self.iter_ptr >= self.vgm.end_offset() || self.hit_invalid_command {
            return None;
        }
        if !self.hit_loop_offset && self.iter_ptr == self.vgm.loop_offset() {
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

                let block: Vec<u8> = (0..block_size)
                    .map(|_| self.iter_u8())
                    .collect();
                Some(VgmIterItem::DataBlock(block_type, block))
            }
            // LR35902 register write
            0xB3 => {
                let addr = self.iter_u8();
                let val = self.iter_u8();
                Some(VgmIterItem::WriteLR35902RegCommand((addr & 0x80) != 0, ((addr & 0x7F) as u16) + 0xFF10, val))
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
