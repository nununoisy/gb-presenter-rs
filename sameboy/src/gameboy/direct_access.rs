use anyhow::{Result, Context};
use std::marker::PhantomData;
use std::ops::{Deref, DerefMut};
use std::{ptr, slice};
use sameboy_sys::{GB_direct_access_t, GB_direct_access_t_GB_DIRECT_ACCESS_BGP, GB_direct_access_t_GB_DIRECT_ACCESS_BOOTROM, GB_direct_access_t_GB_DIRECT_ACCESS_CART_RAM, GB_direct_access_t_GB_DIRECT_ACCESS_HRAM, GB_direct_access_t_GB_DIRECT_ACCESS_IE, GB_direct_access_t_GB_DIRECT_ACCESS_IO, GB_direct_access_t_GB_DIRECT_ACCESS_OAM, GB_direct_access_t_GB_DIRECT_ACCESS_OBP, GB_direct_access_t_GB_DIRECT_ACCESS_RAM, GB_direct_access_t_GB_DIRECT_ACCESS_ROM, GB_direct_access_t_GB_DIRECT_ACCESS_ROM0, GB_direct_access_t_GB_DIRECT_ACCESS_VRAM, GB_get_direct_access};
use super::Gameboy;

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum DirectAccessType {
    ROM,
    RAM,
    CartridgeRAM,
    VRAM,
    HRAM,
    IO,
    BootROM,
    OAM,
    BGP,
    OBP,
    IE,
    ROM0
}

impl From<GB_direct_access_t> for DirectAccessType {
    fn from(value: GB_direct_access_t) -> Self {
        match value {
            GB_direct_access_t_GB_DIRECT_ACCESS_ROM => Self::ROM,
            GB_direct_access_t_GB_DIRECT_ACCESS_RAM => Self::RAM,
            GB_direct_access_t_GB_DIRECT_ACCESS_CART_RAM => Self::CartridgeRAM,
            GB_direct_access_t_GB_DIRECT_ACCESS_VRAM => Self::VRAM,
            GB_direct_access_t_GB_DIRECT_ACCESS_HRAM => Self::HRAM,
            GB_direct_access_t_GB_DIRECT_ACCESS_IO => Self::IO,
            GB_direct_access_t_GB_DIRECT_ACCESS_BOOTROM => Self::BootROM,
            GB_direct_access_t_GB_DIRECT_ACCESS_OAM => Self::OAM,
            GB_direct_access_t_GB_DIRECT_ACCESS_BGP => Self::BGP,
            GB_direct_access_t_GB_DIRECT_ACCESS_OBP => Self::OBP,
            GB_direct_access_t_GB_DIRECT_ACCESS_IE => Self::IE,
            GB_direct_access_t_GB_DIRECT_ACCESS_ROM0 => Self::ROM0,
            _ => unreachable!("Invalid GB_direct_access_t value")
        }
    }
}

impl From<DirectAccessType> for GB_direct_access_t {
    fn from(value: DirectAccessType) -> Self {
        match value {
            DirectAccessType::ROM => GB_direct_access_t_GB_DIRECT_ACCESS_ROM,
            DirectAccessType::RAM => GB_direct_access_t_GB_DIRECT_ACCESS_RAM,
            DirectAccessType::CartridgeRAM => GB_direct_access_t_GB_DIRECT_ACCESS_CART_RAM,
            DirectAccessType::VRAM => GB_direct_access_t_GB_DIRECT_ACCESS_VRAM,
            DirectAccessType::HRAM => GB_direct_access_t_GB_DIRECT_ACCESS_HRAM,
            DirectAccessType::IO => GB_direct_access_t_GB_DIRECT_ACCESS_IO,
            DirectAccessType::BootROM => GB_direct_access_t_GB_DIRECT_ACCESS_BOOTROM,
            DirectAccessType::OAM => GB_direct_access_t_GB_DIRECT_ACCESS_OAM,
            DirectAccessType::BGP => GB_direct_access_t_GB_DIRECT_ACCESS_BGP,
            DirectAccessType::OBP => GB_direct_access_t_GB_DIRECT_ACCESS_OBP,
            DirectAccessType::IE => GB_direct_access_t_GB_DIRECT_ACCESS_IE,
            DirectAccessType::ROM0 => GB_direct_access_t_GB_DIRECT_ACCESS_ROM0
        }
    }
}

pub struct DirectAccess<'a> {
    data: ptr::NonNull<u8>,
    size: usize,
    bank: u16,
    lifetime_marker: PhantomData<&'a ()>
}

unsafe impl Send for DirectAccess<'_> {}
unsafe impl Sync for DirectAccess<'_> {}

impl DirectAccess<'_> {
    pub fn bank(&self) -> u16 {
        self.bank
    }
}

impl<'a> Deref for DirectAccess<'a> {
    type Target = [u8];

    fn deref(&self) -> &'a Self::Target {
        unsafe {
            slice::from_raw_parts(self.data.as_ptr().cast_const(), self.size)
        }
    }
}

impl<'a> DerefMut for DirectAccess<'a> {
    fn deref_mut(&mut self) -> &'a mut Self::Target {
        unsafe {
            slice::from_raw_parts_mut(self.data.as_ptr(), self.size)
        }
    }
}

impl Gameboy {
    pub fn direct_access(&mut self, direct_access_type: DirectAccessType) -> Result<DirectAccess<'_>> {
        unsafe {
            let mut size: usize = 0;
            let mut bank: u16 = 0;
            let data = ptr::NonNull::new(GB_get_direct_access(
                self.as_mut_ptr(),
                direct_access_type.into(),
                ptr::addr_of_mut!(size),
                ptr::addr_of_mut!(bank)
            ) as *mut u8).context("Direct access failed")?;

            Ok(DirectAccess {
                data,
                size,
                bank,
                lifetime_marker: PhantomData
            })
        }
    }
}
