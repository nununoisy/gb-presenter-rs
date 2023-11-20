use std::collections::VecDeque;
use std::marker::PhantomPinned;
use std::sync::{Arc, Mutex};
use std::time::{Instant, SystemTime};
use super::memory::MemoryInterceptor;
use super::audio::{ApuStateReceiver, AUDIO_BUFFER_INITIAL_SIZE};
use super::video::SCREEN_BUF_SIZE;
use super::cartridge::camera::CameraProvider;
use super::cartridge::rumble::RumbleReceiver;
use super::link::LinkTarget;
use super::link::printer::PrinterReceiver;
use super::sgb::SuperGameboyReceiver;

pub(crate) struct GameboyInner {
    pub id: usize,
    pub audio_buf: VecDeque<i16>,
    pub memory_interceptor: Option<Arc<Mutex<dyn MemoryInterceptor>>>,
    pub io_registers_copy: [u8; 0x80],
    pub apu_receiver: Option<Arc<Mutex<dyn ApuStateReceiver>>>,
    pub screen_buf: [u32; SCREEN_BUF_SIZE],
    pub boot_rom_unmapped: bool,
    pub vblank_occurred: bool,
    pub link_target: Option<LinkTarget>,
    pub link_next_bit: bool,
    pub printer_receiver: Option<Arc<Mutex<dyn PrinterReceiver>>>,
    pub workboy_time_base: SystemTime,
    pub workboy_time_last_set: Instant,
    pub camera_provider: Option<Arc<Mutex<dyn CameraProvider>>>,
    pub rumble_receiver: Option<Arc<Mutex<dyn RumbleReceiver>>>,
    pub sgb_receiver: Option<Arc<Mutex<dyn SuperGameboyReceiver>>>,
    _pin: PhantomPinned
}

impl GameboyInner {
    pub fn new(id: usize) -> Self {
        Self {
            id,
            audio_buf: VecDeque::with_capacity(AUDIO_BUFFER_INITIAL_SIZE),
            memory_interceptor: None,
            io_registers_copy: [0u8; 0x80],
            apu_receiver: None,
            screen_buf: [0u32; SCREEN_BUF_SIZE],
            boot_rom_unmapped: false,
            vblank_occurred: false,
            link_target: None,
            link_next_bit: true,
            printer_receiver: None,
            workboy_time_base: SystemTime::now(),
            workboy_time_last_set: Instant::now(),
            camera_provider: None,
            rumble_receiver: None,
            sgb_receiver: None,
            _pin: PhantomPinned
        }
    }
}
