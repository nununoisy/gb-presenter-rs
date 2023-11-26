mod inner;
mod model;
mod memory;
mod audio;
mod video;
mod joypad;
mod cartridge;
mod gbs_info;
mod link;
mod save_state;
mod rtc;
mod sgb;
mod bootrom;
mod direct_access;

use std::sync::atomic::Ordering;
use anyhow::{bail, Result};
use sameboy_sys::{GB_alloc, GB_dealloc, GB_gameboy_t, GB_get_user_data, GB_get_usual_frame_rate, GB_init, GB_reset, GB_run, GB_run_frame, GB_set_user_data};
use inner::GameboyInner;

pub use joypad::JoypadButton;
pub use audio::{ApuChannel, ApuStateReceiver, HighpassFilterMode};
pub use model::{Model, Revision, VideoStandard};
pub use memory::MemoryInterceptor;
pub use gbs_info::GbsInfo;
pub use cartridge::camera::CameraProvider;
pub use cartridge::rumble::{RumbleMode, RumbleReceiver};
pub use link::printer::PrinterReceiver;
pub use link::workboy_key::WorkboyKey;
use crate::gameboy::video::SCREEN_BUF_SIZE;

pub struct Gameboy {
    gb: *mut GB_gameboy_t,
    is_owned: bool
}

#[cfg(feature = "thread-safety")]
unsafe impl Send for Gameboy {}
#[cfg(feature = "thread-safety")]
unsafe impl Sync for Gameboy {}

impl Gameboy {
    #[inline]
    pub unsafe fn wrap(gb: *mut GB_gameboy_t) -> Self {
        Self {
            gb,
            is_owned: false
        }
    }

    #[inline(always)]
    pub unsafe fn as_ptr(&self) -> *const GB_gameboy_t {
        self.gb.cast_const()
    }

    #[inline(always)]
    pub unsafe fn as_mut_ptr(&mut self) -> *mut GB_gameboy_t {
        self.gb
    }

    #[inline(always)]
    pub(crate) unsafe fn inner(&self) -> *const GameboyInner {
        GB_get_user_data(self.gb).cast_const() as *const GameboyInner
    }

    #[inline(always)]
    pub(crate) unsafe fn inner_mut(&mut self) -> *mut GameboyInner {
        GB_get_user_data(self.gb) as *mut GameboyInner
    }
}

impl Gameboy {
    pub fn new(id: usize, model: Model) -> Result<Self> {
        unsafe {
            let gb = GB_alloc();
            if gb.is_null() {
                bail!("GB_alloc() failed");
            }

            let inner_box = Box::new(GameboyInner::new(id));
            let inner_ptr = Box::into_raw(inner_box);

            let screen_buf_box: Box<[u32]> = Box::new([0u32; SCREEN_BUF_SIZE]);
            let screen_buf_ptr = Box::into_raw(screen_buf_box) as *mut u32;

            GB_init(gb, model.into());
            GB_set_user_data(gb, inner_ptr as *mut _);

            Self::init_audio(gb);
            Self::init_video(gb, screen_buf_ptr);
            Self::init_memory(gb);
            Self::init_camera(gb);
            Self::init_rumble(gb);
            #[cfg(feature = "include-bootroms")]
            Self::init_bootrom(gb);

            Ok(Self {
                gb,
                is_owned: true
            })
        }
    }

    /// Get this Gameboy's ID.
    pub fn id(&self) -> usize {
        unsafe {
            (*self.inner()).id
        }
    }

    /// Reset the emulation.
    pub fn reset(&mut self) {
        unsafe {
            (*self.inner_mut()).boot_rom_unmapped.store(false, Ordering::Release);
            GB_reset(self.as_mut_ptr());
        }
    }

    /// Run for a single clock cycle. Returns the number of 8MHz ticks passed.
    pub fn run(&mut self) -> usize {
        unsafe {
            GB_run(self.as_mut_ptr()) as usize
        }
    }

    /// Run for a single frame. Returns the time passed in nanoseconds.
    pub fn run_frame(&mut self) -> u64 {
        unsafe {
            GB_run_frame(self.as_mut_ptr())
        }
    }

    /// Run for a single frame in sync with another Gameboy. Use this if you
    /// connect two consoles with a virtual link cable.
    pub fn run_frame_sync(&mut self, other: &mut Gameboy) {
        unsafe {
            (*self.inner_mut()).vblank_occurred.store(false, Ordering::Release);
            (*other.inner_mut()).vblank_occurred.store(false, Ordering::Release);

            let mut delta = 0i64;
            while !(*self.inner()).vblank_occurred.load(Ordering::Acquire) || !(*other.inner()).vblank_occurred.load(Ordering::Acquire) {
                if delta >= 0 {
                    delta -= self.run() as i64;
                } else {
                    delta += other.run() as i64;
                }
            }
        }
    }

    /// Get the usual frame rate.
    pub fn usual_frame_rate(&mut self) -> f64 {
        unsafe {
            GB_get_usual_frame_rate(self.as_mut_ptr())
        }
    }
}

impl Drop for Gameboy {
    fn drop(&mut self) {
        if !self.is_owned {
            return;
        }

        unsafe {
            let inner_ptr = GB_get_user_data(self.as_mut_ptr()) as *mut GameboyInner;
            let inner_box = Box::from_raw(inner_ptr);
            drop(inner_box);

            GB_dealloc(self.as_mut_ptr());
        }
    }
}
