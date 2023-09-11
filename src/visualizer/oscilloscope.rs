use ringbuf::Rb;
use ringbuf::ring_buffer::RbBase;
use raqote::{AntialiasMode, BlendMode, Color, DrawOptions, DrawTarget, Gradient, GradientStop, LineCap, LineJoin, PathBuilder, Point, SolidSource, Source, Spread, StrokeStyle, Transform};
use sameboy::ApuChannel;
use super::{Visualizer, ChannelState};

const DIVIDER_WIDTH: u32 = 5;

impl Visualizer {
    fn oscilloscope_window(&self, channel: ApuChannel, window_size: usize) -> (Vec<f32>, ChannelState) {
        let buf = match channel {
            ApuChannel::Pulse1 => &self.pulse1_states,
            ApuChannel::Pulse2 => &self.pulse2_states,
            ApuChannel::Wave => &self.wave_states,
            ApuChannel::Noise => &self.noise_states
        };

        if buf.is_empty() {
            return (vec![0.0f32; window_size], ChannelState::default());
        } else if buf.len() <= window_size {
            let mut result = vec![0.0f32; window_size - buf.len()];
            result.extend(buf.iter().map(|s| s.amplitude));
            return (result, buf.iter().last().cloned().unwrap());
        }

        // Perform edge detection:
        let edge_detect_end = buf.len() - window_size;
        let mut edge_buffer: Vec<_> = buf.iter().collect();
        edge_buffer.truncate(edge_detect_end);
        // let min_sample = edge_buffer.iter()
        //     .map(|s| s.amplitude)
        //     .reduce(f32::min)
        //     .unwrap_or(0.0);
        // let max_sample = edge_buffer.iter()
        //     .map(|s| s.amplitude)
        //     .reduce(f32::max)
        //     .unwrap_or(0.0);
        // let edge_threshold = (min_sample + max_sample) / 2.0;
        // let edge_index = edge_buffer.windows(2)
        //     // Convolve the function f: amplitude > threshold with the kernel g: [1, -1]
        //     // to detect edges
        //     .map(|w| {
        //         let f0 = (w[0].amplitude > edge_threshold) as i16;
        //         let f1 = (w[1].amplitude > edge_threshold) as i16;
        //         f0 - f1
        //     })
        //     .rposition(|s| s == -1);
        let edge_index = edge_buffer.iter().rposition(|s| s.edge);

        let start_index = match edge_index {
            // Center the graph on the rising edge of the amplitude
            Some(edge_index) => edge_index.saturating_sub(window_size / 2),
            // If no edge was found, just use the latest window
            None => edge_detect_end
        };
        let end_index = std::cmp::min(start_index + window_size, buf.len());

        let samples: Vec<_> = buf.iter()
            .enumerate()
            .filter_map(|(i, s)| {
                if (start_index..end_index).contains(&i) {
                    Some(s.amplitude)
                } else {
                    None
                }
            })
            .collect();

        (samples, buf.iter().last().cloned().unwrap())
    }

    pub fn draw_oscilloscope_view(&mut self, channel: ApuChannel, x: f32, y: f32, w: f32, h: f32) {
        let settings = self.settings.settings(channel);
        let (window, last_state) = self.oscilloscope_window(channel, (w * 2.0) as _);

        let mut pb = PathBuilder::new();
        for (i, s) in window.iter().enumerate() {
            let px = (i as f32) / 2.0;
            let py = (15.0 - *s) * h / 30.0;

            if i == 0 {
                pb.move_to(x + px, y + py);
            } else {
                pb.line_to(x + px, y + py);
            }
        }
        let path = pb.finish();

        let color = settings.color(&last_state).unwrap();

        let bg_color = Color::new(0x20, color.r(), color.g(), color.b());
        let bg_source = Source::new_linear_gradient(
            Gradient {
                stops: vec![
                    GradientStop { position: 0.0, color: bg_color },
                    GradientStop { position: 0.5, color: Color::new(0x20, 0, 0, 0) },
                    GradientStop { position: 1.0, color: bg_color }
                ],
            },
            Point::new(w / 2.0, 0.0),
            Point::new(w / 2.0, h),
            Spread::Pad
        );
        self.canvas.fill_rect(
            x, y, w, h,
            &Source::from(Color::new(0xFF, 0, 0, 0)),
            &DrawOptions::new()
        );
        if last_state.balance <= 0.5 {
            self.canvas.fill_rect(
                x, y, w / 2.0, h,
                &bg_source,
                &DrawOptions::new()
            );
        }
        if last_state.balance >= 0.5 {
            self.canvas.fill_rect(
                x + (w / 2.0), y, w / 2.0, h,
                &bg_source,
                &DrawOptions::new()
            );
        }

        let padding = (self.font.tile_h() as f32) / 2.0;
        let name_width = (self.font.tile_w() * settings.name().len()) as f32;
        self.font.draw_text(&mut self.canvas, "LR35902", x + padding + (DIVIDER_WIDTH / 2) as f32, y + padding, 0.2);
        self.font.draw_text(&mut self.canvas, &settings.name(), x + w - name_width - padding - DIVIDER_WIDTH as f32, y + h - 3.0 * padding, 0.2);

        let glow_color = Color::new(0x40, color.r(), color.g(), color.b());
        let glow_source = Source::Solid(SolidSource::from(glow_color));
        self.canvas.stroke(
            &path,
            &glow_source,
            &StrokeStyle {
                width: 3.0,
                cap: LineCap::Round,
                join: LineJoin::Round,
                miter_limit: 2.0,
                dash_array: vec![],
                dash_offset: 0.0,
            },
            &DrawOptions::default()
        );

        let line_source = Source::Solid(SolidSource::from(color));
        self.canvas.stroke(
            &path,
            &line_source,
            &StrokeStyle {
                width: 1.0,
                cap: LineCap::Round,
                join: LineJoin::Round,
                miter_limit: 2.0,
                dash_array: vec![],
                dash_offset: 0.0,
            },
            &DrawOptions::default()
        );

        for dx in 0..DIVIDER_WIDTH {
            let gradient_index = (255 * (DIVIDER_WIDTH - dx)) / DIVIDER_WIDTH;
            let gradient_color = Color::new(((gradient_index * gradient_index) / 255) as u8, 0, 0, 0);
            let gradient_source = Source::Solid(SolidSource::from(gradient_color));

            self.canvas.fill_rect(
                x - 1.0 + dx as f32, y, 1.0, h,
                &gradient_source,
                &DrawOptions::new()
            );
            self.canvas.fill_rect(
                x + w - 1.0 - dx as f32, y, 1.0, h,
                &gradient_source,
                &DrawOptions::new()
            );
        }
    }

    pub fn draw_oscilloscopes(&mut self, x: f32, y: f32, w: f32, h: f32) {
        if self.is_vertical_layout() {
            let scope_w = w / 2.0;
            let scope_h = h / 2.0;

            self.draw_oscilloscope_view(ApuChannel::Pulse1, x, y, scope_w, scope_h);
            self.draw_oscilloscope_view(ApuChannel::Pulse2, x + scope_w, y, scope_w, scope_h);
            self.draw_oscilloscope_view(ApuChannel::Wave, x, y + scope_h, scope_w, scope_h);
            self.draw_oscilloscope_view(ApuChannel::Noise, x + scope_w, y + scope_h, scope_w, scope_h);
        } else {
            let scope_w = w / 4.0;

            self.draw_oscilloscope_view(ApuChannel::Pulse1, x, y, scope_w, h);
            self.draw_oscilloscope_view(ApuChannel::Pulse2, x + scope_w, y, scope_w, h);
            self.draw_oscilloscope_view(ApuChannel::Wave, x + (2.0 * scope_w), y, scope_w, h);
            self.draw_oscilloscope_view(ApuChannel::Noise, x + (3.0 * scope_w), y, scope_w, h);
        }
    }
}