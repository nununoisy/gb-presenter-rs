mod filters;
pub mod channel_settings;
mod oscilloscope;
mod piano_roll;
mod tile_map;

use tiny_skia::{Color, Pixmap, Rect};
use channel_settings::{ChannelSettingsManager, ChannelSettings};
use filters::HighPassIIR;
use oscilloscope::OscilloscopeState;
use piano_roll::PianoRollState;
use sameboy::{ApuChannel, ApuStateReceiver};
use tile_map::TileMap;
use crate::config::PianoRollConfig;

pub const APU_STATE_BUF_SIZE: usize = 4096;
const FONT_IMAGE: &'static [u8] = include_bytes!("8x8_font.png");
const FONT_CHAR_MAP: &'static str = " !\"#$%&'()*+,-./0123456789:;<=>?@ABCDEFGHIJKLMNOPQRSTUVWXYZ[\\]^_`abcdefghijklmnopqrstuvwxyz{|}~";

#[derive(Copy, Clone, Default)]
pub struct ChannelState {
    pub volume: f32,
    pub amplitude: f32,
    pub frequency: f64,
    pub timbre: usize,
    pub balance: f64,
    pub edge: bool
}

pub struct Visualizer {
    channels: usize,
    canvas: Pixmap,
    config: PianoRollConfig,

    channel_last_states: Vec<ChannelState>,
    channel_filters: Vec<HighPassIIR>,
    oscilloscope_states: Vec<OscilloscopeState>,
    piano_roll_states: Vec<PianoRollState>,

    font: TileMap,
    oscilloscope_divider_cache: Option<(f32, Pixmap)>
}

impl Visualizer {
    pub fn new(channels: usize, width: u32, height: u32, sample_rate: u32, config: PianoRollConfig) -> Self {
        let mut oscilloscope_states: Vec<OscilloscopeState> = Vec::with_capacity(channels);
        let mut piano_roll_states: Vec<PianoRollState> = Vec::with_capacity(channels);
        for _ in 0..channels {
            oscilloscope_states.push(OscilloscopeState::new());
            piano_roll_states.push(PianoRollState::new(sample_rate as f32, config.speed_multiplier as f32 * 4.0, config.starting_octave as f32));
        }

        Self {
            channels,
            canvas: Pixmap::new(width, height).unwrap(),
            config,
            channel_last_states: vec![ChannelState::default(); channels],
            channel_filters: vec![HighPassIIR::new(sample_rate as f32, 300.0); channels],
            oscilloscope_states,
            piano_roll_states,
            font: TileMap::new(Pixmap::decode_png(FONT_IMAGE).unwrap(), 8, 8, FONT_CHAR_MAP),
            oscilloscope_divider_cache: None
        }
    }

    pub fn get_canvas_buffer(&self) -> &[u8] {
        self.canvas.data()
    }

    pub fn clear(&mut self) {
        self.canvas.fill(Color::TRANSPARENT);
    }

    pub fn draw(&mut self) {
        self.clear();

        let oscilloscopes_pos = Rect::from_xywh(
            0.0,
            0.0,
            self.canvas.width() as f32,
            self.config.waveform_height as f32
        ).unwrap();

        let max_channels_per_row = if self.is_vertical_layout() {
            4
        } else {
            8
        };
        self.draw_oscilloscopes(oscilloscopes_pos, max_channels_per_row);

        let piano_roll_pos = Rect::from_xywh(
            0.0,
            oscilloscopes_pos.bottom(),
            self.canvas.width() as f32,
            self.canvas.height() as f32 - oscilloscopes_pos.height()
        ).unwrap();

        self.draw_piano_roll(piano_roll_pos);
    }

    pub fn settings_manager(&self) -> &ChannelSettingsManager {
        &self.config.settings
    }

    pub fn settings_manager_mut(&mut self) -> &mut ChannelSettingsManager {
        &mut self.config.settings
    }

    pub fn is_vertical_layout(&self) -> bool {
        self.canvas.height() > self.canvas.width()
    }
}

impl ApuStateReceiver for Visualizer {
    fn receive(&mut self, id: usize, channel: ApuChannel, volume: u8, amplitude: u8, frequency: f64, timbre: usize, balance: f64, edge: bool) {
        let frequency = match channel {
            ApuChannel::Noise => frequency,
            _ => frequency * 2.0
        };

        let volume = match channel {
            ApuChannel::Wave => {
                if volume > 0 {
                    (2.0 * volume as f32 + amplitude as f32) / 3.0
                } else {
                    0.0
                }
            },
            _ => volume as f32
        };

        let channel = (id * 4) + match channel {
            ApuChannel::Pulse1 => 0,
            ApuChannel::Pulse2 => 1,
            ApuChannel::Wave => 2,
            ApuChannel::Noise => 3
        };

        let settings = self.config.settings.settings(channel).unwrap();
        let timbre_max = settings.num_colors();

        let filter = self.channel_filters.get_mut(channel).unwrap();
        filter.consume(amplitude as f32);

        let state = ChannelState {
            volume,
            amplitude: filter.output(),
            frequency,
            timbre: timbre % timbre_max,
            balance,
            edge,
        };

        self.oscilloscope_states[channel].consume(&state, settings);
        self.piano_roll_states[channel].consume(&state, settings);
        self.channel_last_states[channel] = state;
    }
}
