mod oscilloscope;
mod channel_settings;
mod tile_map;
mod filters;
mod piano_roll;

use ringbuf::{HeapRb, Rb};
use sameboy::{ApuChannel, ApuStateReceiver};
use raqote::{DrawTarget, SolidSource};
use crate::visualizer::channel_settings::ChannelSettingsManager;
use crate::visualizer::filters::HighPassIIR;
use crate::visualizer::tile_map::TileMap;

#[derive(Copy, Clone, Default)]
pub struct ChannelState {
    pub channel: ApuChannel,
    pub volume: u8,
    pub amplitude: f32,
    pub frequency: f64,
    pub timbre: usize,
    pub balance: f64,
    pub edge: bool
}

pub struct Visualizer {
    canvas: DrawTarget,
    settings: ChannelSettingsManager,
    pulse1_states: HeapRb<ChannelState>,
    pulse1_iir: HighPassIIR,
    pulse2_states: HeapRb<ChannelState>,
    pulse2_iir: HighPassIIR,
    wave_states: HeapRb<ChannelState>,
    wave_iir: HighPassIIR,
    noise_states: HeapRb<ChannelState>,
    noise_iir: HighPassIIR,
    state_slices: HeapRb<ChannelState>,
    font: TileMap
}

const APU_STATE_BUF_SIZE: usize = 8192;
const FONT_IMAGE: &'static [u8] = include_bytes!("8x8_font.png");
const FONT_CHAR_MAP: &'static str = " !\"#$%&'()*+,-./0123456789:;<=>?@ABCDEFGHIJKLMNOPQRSTUVWXYZ[\\]^_`abcdefghijklmnopqrstuvwxyz{|}~";

impl Visualizer {
    pub fn new() -> Self {
        Self {
            canvas: DrawTarget::new(960, 540),
            settings: ChannelSettingsManager::default(),
            pulse1_states: HeapRb::new(APU_STATE_BUF_SIZE),
            pulse1_iir: HighPassIIR::new(44100.0, 300.0),
            pulse2_states: HeapRb::new(APU_STATE_BUF_SIZE),
            pulse2_iir: HighPassIIR::new(44100.0, 300.0),
            wave_states: HeapRb::new(APU_STATE_BUF_SIZE),
            wave_iir: HighPassIIR::new(44100.0, 300.0),
            noise_states: HeapRb::new(APU_STATE_BUF_SIZE),
            noise_iir: HighPassIIR::new(44100.0, 300.0),
            state_slices: HeapRb::new(APU_STATE_BUF_SIZE),
            font: TileMap::new(FONT_IMAGE, 8, 8, FONT_CHAR_MAP).unwrap()
        }
    }

    /// Get canvas buffer as BGRA data (little endian) or ARGB data (big endian)
    pub fn get_canvas_buffer(&self) -> Vec<u8> {
        self.canvas.get_data_u8().to_vec()
    }

    pub fn clear(&mut self) {
        self.canvas.clear(SolidSource::from_unpremultiplied_argb(0, 0, 0, 0));
    }
}

impl ApuStateReceiver for Visualizer {
    fn receive(&mut self, channel: ApuChannel, volume: u8, amplitude: u8, frequency: f64, timbre: usize, balance: f64, edge: bool) {
        let (buf, filter)  = match channel {
            ApuChannel::Pulse1 => (&mut self.pulse1_states, &mut self.pulse1_iir),
            ApuChannel::Pulse2 => (&mut self.pulse2_states, &mut self.pulse2_iir),
            ApuChannel::Wave => (&mut self.wave_states, &mut self.wave_iir),
            ApuChannel::Noise => (&mut self.noise_states, &mut self.noise_iir)
        };

        filter.consume(amplitude as f32);
        // Average the wave volume with the amplitude so the
        // wave pattern shows up in the piano roll slices
        let volume = match channel {
            ApuChannel::Wave => (amplitude / 3) + (2 * volume / 3),
            _ => volume
        };

        let timbre_max = self.settings.settings(channel).num_colors();

        let state = ChannelState {
            channel,
            volume,
            amplitude: filter.output(),
            frequency,
            timbre: timbre % timbre_max,
            balance,
            edge
        };

        buf.push_overwrite(state);
    }
}