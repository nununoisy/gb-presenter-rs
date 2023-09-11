use raqote::{AntialiasMode, BlendMode, Color, DrawOptions, PathBuilder, SolidSource, Source};
use ringbuf::Rb;
use sameboy::ApuChannel;
use crate::visualizer::ChannelState;
use super::Visualizer;

const KEY_COUNT: usize = 108;
const KEY_THICKNESS: f32 = 5.0;
const KEY_HEIGHT: f32 = 24.0;

#[derive(Copy, Clone, PartialEq)]
enum PianoKey {
    WhiteLeft,
    WhiteCenter,
    WhiteRight,
    Black
}

const PIANO_KEYS: [PianoKey; 12] = [
    PianoKey::WhiteLeft,    // C
    PianoKey::Black,        // C#
    PianoKey::WhiteCenter,  // D
    PianoKey::Black,        // D#
    PianoKey::WhiteRight,   // E
    PianoKey::WhiteLeft,    // F
    PianoKey::Black,        // F#
    PianoKey::WhiteCenter,  // G
    PianoKey::Black,        // G#
    PianoKey::WhiteCenter,  // A
    PianoKey::Black,        // A#
    PianoKey::WhiteRight    // B
];
const C_0: f64 = 16.351597831287;

impl Visualizer {
    fn draw_piano_key(&mut self, key: PianoKey, x: f32, y: f32, w: f32, h: f32, color: Option<Color>) {
        let key_source = match (color, key) {
            (Some(color), _) => Source::Solid(SolidSource::from(color)),
            (None, PianoKey::Black) => Source::Solid(SolidSource::from_unpremultiplied_argb(0xff, 0x00, 0x00, 0x00)),
            (None, _) => Source::Solid(SolidSource::from_unpremultiplied_argb(0xff, 0x20, 0x20, 0x20))
        };

        let draw_options = DrawOptions {
            blend_mode: BlendMode::SrcOver,
            alpha: 1.0,
            antialias: AntialiasMode::None,
        };

        // TODO convert to path-based rendering instead
        match key {
            PianoKey::WhiteLeft => {
                self.canvas.fill_rect(
                    x - (w / 2.0) + 1.0,
                    y + 1.0,
                    w - 1.0,
                    h - 1.0,
                    &key_source,
                    &draw_options
                );
                self.canvas.fill_rect(
                    x + (w / 2.0),
                    y + (h / 2.0) + 1.0,
                    w / 2.0,
                    (h / 2.0) - 1.0,
                    &key_source,
                    &draw_options
                );
            },
            PianoKey::WhiteCenter => {
                self.canvas.fill_rect(
                    x - (w / 2.0) + 1.0,
                    y + 1.0,
                    w - 1.0,
                    h / 2.0,
                    &key_source,
                    &draw_options
                );
                self.canvas.fill_rect(
                    x - w + 1.0,
                    y + (h / 2.0) + 1.0,
                    (w * 2.0) - 1.0,
                    (h / 2.0) - 1.0,
                    &key_source,
                    &draw_options
                );
            },
            PianoKey::WhiteRight => {
                self.canvas.fill_rect(
                    x - (w / 2.0) + 1.0,
                    y + 1.0,
                    w - 1.0,
                    h - 1.0,
                    &key_source,
                    &draw_options
                );
                self.canvas.fill_rect(
                    x - w + 1.0,
                    y + (h / 2.0) + 1.0,
                    w / 2.0,
                    (h / 2.0) - 1.0,
                    &key_source,
                    &draw_options
                );
            },
            PianoKey::Black => {
                self.canvas.fill_rect(
                    x - (w / 2.0),
                    y + 1.0,
                    w + 1.0,
                    h / 2.0,
                    &key_source,
                    &draw_options
                );
            }
        }
    }

    fn draw_piano_keys(&mut self, x: f32, y: f32, w: f32, h: f32, key_w: f32) {
        let white_border_source = Source::Solid(SolidSource::from_unpremultiplied_argb(0xff, 0x18, 0x18, 0x18));
        let top_edge_source = Source::Solid(SolidSource::from_unpremultiplied_argb(0xff, 0x04, 0x04, 0x04));

        let keys_w = key_w * KEY_COUNT as f32;
        let keys_x = x + ((w - keys_w) / 2.0);

        self.canvas.fill_rect(x, y, w, h + 1.0, &top_edge_source, &DrawOptions::default());
        self.canvas.fill_rect(keys_x, y, keys_w, h, &white_border_source, &DrawOptions::default());
        for key_i in 0..KEY_COUNT {
            let key_t = PIANO_KEYS[key_i % 12].clone();
            let key_x = keys_x + key_w * key_i as f32;

            self.draw_piano_key(key_t, key_x, y, key_w, h, None);
        }
        self.canvas.fill_rect(x, y, w, 1.0, &top_edge_source, &DrawOptions::default());
    }

    fn draw_channel_key_spot(&mut self, channel: ApuChannel, x: f32, y: f32, w: f32, h: f32, key_w: f32) {
        let last_state = match channel {
            ApuChannel::Pulse1 => self.pulse1_states.iter().last(),
            ApuChannel::Pulse2 => self.pulse2_states.iter().last(),
            ApuChannel::Wave => self.wave_states.iter().last(),
            ApuChannel::Noise => self.noise_states.iter().last()
        };
        if last_state.is_none() {
            return;
        }
        let last_state = last_state.unwrap();

        let settings = self.settings.settings(channel);
        let color = settings.color(&last_state).unwrap();
        let volume_alpha = match last_state.volume {
            0 => return,
            v => 0.5 + (v as f32) / 30.0
        };

        let frequency = match channel {
            ApuChannel::Noise => last_state.frequency,
            _ => last_state.frequency * 2.0
        };
        let n = 12.0 * (frequency / C_0).log2() as f32;
        let octave = (n / 12.0).floor();
        let note = n.rem_euclid(12.0);

        let lower_alpha_multiplier = if note.ceil() != note.floor() {
            note.ceil() - note
        } else {
            1.0
        };

        let lower_note = note.floor();
        let lower_octave = octave;
        let lower_key = PIANO_KEYS[lower_note as usize].clone();
        let lower_alpha = (255.0 * volume_alpha * lower_alpha_multiplier) as u8;
        let lower_color = Color::new(lower_alpha, color.r(), color.g(), color.b());

        let upper_note = note.ceil().rem_euclid(12.0);
        let upper_octave = octave + (note.ceil() / 12.0).floor();
        let upper_key = PIANO_KEYS[upper_note as usize].clone();
        let upper_alpha = (255.0 * volume_alpha * (note - note.floor())) as u8;
        let upper_color = Color::new(upper_alpha, color.r(), color.g(), color.b());

        let keys_w = key_w * KEY_COUNT as f32;
        let keys_x = x + (w / 2.0) - (keys_w / 2.0);

        let lower_x = keys_x + key_w * (lower_note + 12.0 * lower_octave);
        let upper_x = keys_x + key_w * (upper_note + 12.0 * upper_octave);

        self.draw_piano_key(lower_key, lower_x, y, key_w, h, Some(lower_color));
        self.draw_piano_key(upper_key, upper_x, y, key_w, h, Some(upper_color));
    }

    fn get_slices_for_this_frame(&self, channel: ApuChannel) -> Vec<ChannelState> {
        let slice_iter = match channel {
            ApuChannel::Pulse1 => self.pulse1_states.iter(),
            ApuChannel::Pulse2 => self.pulse2_states.iter(),
            ApuChannel::Wave => self.wave_states.iter(),
            ApuChannel::Noise => self.noise_states.iter()
        };

        slice_iter
            .rev()
            .step_by(self.sample_rate as usize / (60 * 4))
            .take(4)
            .cloned()
            .collect()
    }

    fn draw_channel_slices(&mut self, x: f32, y: f32, w: f32, h: f32, key_w: f32) {
        let keys_w = key_w * KEY_COUNT as f32;
        let keys_x = x + (w / 2.0) - (keys_w / 2.0);

        for (i, state) in self.state_slices.iter().rev().enumerate() {
            if (i / 4) > h.floor() as usize {
                break;
            }
            if state.volume == 0 {
                continue;
            }

            let settings = self.settings.settings(state.channel);
            let color = settings.color(&state).unwrap();

            let frequency = match state.channel {
                ApuChannel::Noise => state.frequency,
                _ => state.frequency * 2.0
            };
            let n = 12.0 * (frequency / C_0).log2() as f32;
            let octave = (n / 12.0).floor();
            let note = n.rem_euclid(12.0);

            let slice_w = state.volume as f32;
            let slice_x = keys_x + (key_w * (note + 12.0 * octave)) - (slice_w / 2.0);
            let slice_y = y + (i / 4) as f32;

            self.canvas.fill_rect(
                slice_x - (KEY_THICKNESS / 2.0),
                slice_y,
                slice_w + KEY_THICKNESS,
                1.0 + (KEY_THICKNESS / 2.0),
                &Source::from(Color::new(0xFF, 0, 0, 0)),
                &DrawOptions::default()
            );
            self.canvas.fill_rect(
                slice_x,
                slice_y,
                slice_w,
                1.0,
                &Source::Solid(SolidSource::from(color)),
                &DrawOptions::default()
            );
        }
    }

    pub fn draw_piano_roll(&mut self, x: f32, y: f32, w: f32, h: f32) {
        self.draw_piano_keys(x, y, w, KEY_HEIGHT, KEY_THICKNESS);
        self.draw_channel_key_spot(ApuChannel::Noise, x, y, w, KEY_HEIGHT, KEY_THICKNESS);
        self.draw_channel_key_spot(ApuChannel::Wave, x, y, w, KEY_HEIGHT, KEY_THICKNESS);
        self.draw_channel_key_spot(ApuChannel::Pulse2, x, y, w, KEY_HEIGHT, KEY_THICKNESS);
        self.draw_channel_key_spot(ApuChannel::Pulse1, x, y, w, KEY_HEIGHT, KEY_THICKNESS);

        let slices_y = y + KEY_HEIGHT;
        let slices_h = h - KEY_HEIGHT;

        let p1_slices = self.get_slices_for_this_frame(ApuChannel::Pulse1);
        let p2_slices = self.get_slices_for_this_frame(ApuChannel::Pulse2);
        let n_slices = self.get_slices_for_this_frame(ApuChannel::Noise);
        let w_slices = self.get_slices_for_this_frame(ApuChannel::Wave);

        (0..4)
            .flat_map(|i| vec![p1_slices[i], p2_slices[i], n_slices[i], w_slices[i]])
            .rev()
            .for_each(|s| {
                self.state_slices.push_overwrite(s);
            });

        self.draw_channel_slices(x, slices_y, w, slices_h, KEY_THICKNESS);
    }
}
