use std::collections::VecDeque;
use std::marker::PhantomPinned;
use std::sync::{Arc, Mutex};
use std::sync::atomic::AtomicBool;
use std::time::{Instant, SystemTime};
use crate::ApuChannel;
use super::memory::MemoryInterceptor;
use super::audio::{ApuStateReceiver, AUDIO_BUFFER_INITIAL_SIZE};
use super::cartridge::camera::CameraProvider;
use super::cartridge::rumble::RumbleReceiver;
use super::link::LinkTarget;
use super::link::printer::PrinterReceiver;
use super::sgb::SuperGameboyReceiver;

pub(crate) struct GameboyInner {
    // The ID is never modified after the inner struct is created, so this can remain non-atomic
    pub id: usize,
    pub audio_buf: Arc<Mutex<VecDeque<i16>>>,
    pub memory_interceptor: Arc<Mutex<dyn MemoryInterceptor>>,
    pub io_registers_copy: Arc<Mutex<[u8; 0x80]>>,
    pub apu_receiver: Arc<Mutex<dyn ApuStateReceiver>>,
    pub rendering_disabled: AtomicBool,
    pub boot_rom_unmapped: AtomicBool,
    pub vblank_occurred: AtomicBool,
    pub link_target: Arc<Mutex<LinkTarget>>,
    pub link_next_bit: AtomicBool,
    pub printer_receiver: Arc<Mutex<dyn PrinterReceiver>>,
    pub workboy_time_base: Arc<Mutex<SystemTime>>,
    pub workboy_time_last_set: Arc<Mutex<Instant>>,
    pub camera_provider: Arc<Mutex<dyn CameraProvider>>,
    pub rumble_receiver: Arc<Mutex<dyn RumbleReceiver>>,
    pub sgb_receiver: Arc<Mutex<dyn SuperGameboyReceiver>>,
    _pin: PhantomPinned
}

pub(crate) struct Dummy;

impl MemoryInterceptor for Dummy {}
impl ApuStateReceiver for Dummy {
    fn receive(&mut self, _id: usize, _channel: ApuChannel, _volume: u8, _amplitude: u8, _frequency: f64, _timbre: usize, _balance: f64, _edge: bool) {}
}
impl PrinterReceiver for Dummy {
    fn print_data_updated(&mut self, _id: usize, _image: &[u32], _top_margin: u8, _bottom_margin: u8, _exposure: u8) {}
    fn print_finished(&mut self, _id: usize) {}
}
impl CameraProvider for Dummy {
    fn get_pixel(&self, _id: usize, _x: u8, _y: u8) -> u8 {
        0
    }
    fn update(&mut self, _id: usize) {}
}
impl RumbleReceiver for Dummy {
    fn receive(&mut self, _id: usize, _amplitude: f64) {}
}
impl SuperGameboyReceiver for Dummy {
    fn joypad_write(&mut self, _id: usize, _value: u8) {}
    fn icd_pixel(&mut self, _id: usize, _row: u8) {}
    fn icd_hreset(&mut self, _id: usize) {}
    fn icd_vreset(&mut self, _id: usize) {}
}

impl GameboyInner {
    pub fn new(id: usize) -> Self {
        Self {
            id,
            audio_buf: Arc::new(Mutex::new(VecDeque::with_capacity(AUDIO_BUFFER_INITIAL_SIZE))),
            memory_interceptor: Arc::new(Mutex::new(Dummy)),
            io_registers_copy: Arc::new(Mutex::new([0u8; 0x80])),
            apu_receiver: Arc::new(Mutex::new(Dummy)),
            rendering_disabled: AtomicBool::new(false),
            boot_rom_unmapped: AtomicBool::new(false),
            vblank_occurred: AtomicBool::new(false),
            link_target: Arc::new(Mutex::new(LinkTarget::None)),
            link_next_bit: AtomicBool::new(true),
            printer_receiver: Arc::new(Mutex::new(Dummy)),
            workboy_time_base: Arc::new(Mutex::new(SystemTime::now())),
            workboy_time_last_set: Arc::new(Mutex::new(Instant::now())),
            camera_provider: Arc::new(Mutex::new(Dummy)),
            rumble_receiver: Arc::new(Mutex::new(Dummy)),
            sgb_receiver: Arc::new(Mutex::new(Dummy)),
            _pin: PhantomPinned
        }
    }
}
