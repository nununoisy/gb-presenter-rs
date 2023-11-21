use std::collections::HashMap;
use std::iter;
use ringbuf::{HeapRb, Rb, ring_buffer::RbBase};
use tiny_skia::{Color, GradientStop, LinearGradient, LineCap, LineJoin, Paint, PathBuilder, Pixmap, PixmapPaint, Point, Rect, SpreadMode, Stroke, Transform};
use super::{Visualizer, APU_STATE_BUF_SIZE, ChannelState, ChannelSettings};

pub struct OscilloscopeState {
    amplitudes: HeapRb<f32>,
    edges: HeapRb<bool>,
    background_cache: HashMap<[u8; 3], Pixmap>
}

impl OscilloscopeState {
    pub fn new() -> Self {
        Self {
            amplitudes: HeapRb::new(APU_STATE_BUF_SIZE),
            edges: HeapRb::new(APU_STATE_BUF_SIZE),
            background_cache: HashMap::new()
        }
    }

    pub fn consume(&mut self, state: ChannelState, _settings: &ChannelSettings) {
        self.amplitudes.push_overwrite(state.amplitude);
        self.edges.push_overwrite(state.edge);
    }
}

impl Visualizer {
    fn oscilloscope_edge_detect(&self, channel: usize, window_size: usize) -> Box<dyn Iterator<Item=&f32> + '_> {
        let state = self.oscilloscope_states.get(channel).unwrap();

        if state.amplitudes.is_empty() {
            return Box::new(
                iter::repeat(&0.0_f32)
                    .take(window_size)
            );
        } else if state.amplitudes.len() <= window_size {
            return Box::new(
                iter::repeat(&0.0_f32)
                    .take(window_size - state.amplitudes.len())
                    .chain(state.amplitudes.iter())
            );
        }

        let edge_detect_end = state.amplitudes.len() - window_size;

        // We can't use rev()/rposition() here because the ring buffer iterator doesn't
        // impl ExactSizeIterator. Just use a forward loop to avoid needlessly cloning.
        let mut edge_index: Option<usize> = None;
        for (i, edge) in state.edges.iter().take(edge_detect_end).enumerate() {
            if *edge {
                edge_index = Some(i);
            }
        }

        let start_index = match edge_index {
            // Center the graph on the rising edge of the amplitude
            Some(edge_index) => edge_index.saturating_sub(window_size / 2),
            // If no edge was found, just use the latest window
            None => edge_detect_end
        };
        let end_index = std::cmp::min(start_index + window_size, state.amplitudes.len());

        Box::new(
            state.amplitudes
                .iter()
                .skip(start_index)
                .take(end_index - start_index)
        )
    }

    fn oscilloscope_window(&self, channel: usize, window_size: usize) -> Vec<(f32, u32)> {
        let mut result: Vec<(f32, u32)> = Vec::with_capacity(window_size / 4);

        for amplitude in self.oscilloscope_edge_detect(channel, window_size) {
            if let Some(last_result) = result.last_mut() {
                if last_result.0 == *amplitude {
                    last_result.1 += 1;
                    continue;
                }
            }

            result.push((*amplitude, 1));
        }

        let last_amplitude = result.last()
            .map(|(amplitude, _len)| *amplitude)
            .unwrap_or(0.0);
        result.push((last_amplitude, 0));

        result
    }

    pub fn draw_oscilloscope_view(&mut self, channel: usize, pos: Rect) {
        let settings = self.config.settings.settings(channel).unwrap();
        let window = self.oscilloscope_window(channel, (pos.width() * 2.0) as _);
        let last_state = self.channel_last_states[channel];

        let color = settings.color(&last_state).unwrap();

        let mut pb = PathBuilder::new();
        let mut px = 0.0_f32;
        for (i, (s, w)) in window.iter().enumerate() {
            let py = (15.0 - *s) * pos.height() / 30.0;

            if i == 0 {
                pb.move_to(pos.x() + px, pos.y() + py);
            } else {
                pb.line_to(pos.x() + px, pos.y() + py);
            }

            px += (*w as f32) / 2.0;
        }
        let path = pb.finish().unwrap();

        let background_cache = &mut self.oscilloscope_states.get_mut(channel).unwrap().background_cache;
        let color_u8 = color.to_color_u8();
        let cache_key = [color_u8.red(), color_u8.green(), color_u8.blue()];
        let cache_valid = background_cache.get(&cache_key)
            .map(|p| p.width() == (pos.width() / 2.0) as u32 && p.height() == pos.height() as u32)
            .unwrap_or(false);
        if !cache_valid {
            let mut cache_pixmap = Pixmap::new((pos.width() / 2.0) as u32, pos.height() as u32).unwrap();

            let bg_color = Color::from_rgba(color.red(), color.green(), color.blue(), 0.125).unwrap();
            let mut bg_paint = Paint::default();
            bg_paint.anti_alias = false;
            bg_paint.shader = LinearGradient::new(
                Point::from_xy(0.0, 0.0),
                Point::from_xy(0.0, pos.height()),
                vec![
                    GradientStop::new(0.0, bg_color),
                    GradientStop::new(0.5, Color::from_rgba8(0, 0, 0, 0x20)),
                    GradientStop::new(1.0, bg_color)
                ],
                SpreadMode::Pad,
                Transform::identity()
            ).unwrap();

            cache_pixmap.fill_rect(
                Rect::from_xywh(0.0, 0.0, pos.width() / 2.0, pos.height()).unwrap(),
                &bg_paint,
                Transform::identity(),
                None
            );

            background_cache.insert(cache_key.clone(), cache_pixmap);
        }
        let background = background_cache.get(&cache_key).unwrap().as_ref();

        if last_state.balance <= 0.5 {
            self.canvas.draw_pixmap(
                pos.x() as i32,
                pos.y() as i32,
                background,
                &PixmapPaint::default(),
                Transform::identity(),
                None
            );
        }
        if last_state.balance >= 0.5 {
            self.canvas.draw_pixmap(
                (pos.x() + (pos.width() / 2.0)) as i32,
                pos.y() as i32,
                background,
                &PixmapPaint::default(),
                Transform::identity(),
                None
            );
        }

        if self.config.draw_text_labels {
            let text_padding = (self.font.tile_h() as f32) / 2.0;
            let chip_name_pos = Point::from_xy(
                pos.x() + text_padding + (self.config.divider_width as f32 / 2.0),
                pos.y() + text_padding
            );
            let channel_name_width = (self.font.tile_w() * settings.name().chars().count()) as f32;
            let channel_name_pos = Point::from_xy(
                pos.x() + pos.width() - channel_name_width - text_padding - self.config.divider_width as f32,
                pos.y() + pos.height() - 3.0 * text_padding
            );

            self.font.draw_text(&mut self.canvas.as_mut(), &settings.chip(), chip_name_pos, 0.2);
            self.font.draw_text(&mut self.canvas.as_mut(), &settings.name(), channel_name_pos, 0.2);
        }

        let glow_color = Color::from_rgba(color.red(), color.green(), color.blue(), 0.25).unwrap();
        let mut glow_paint = Paint::default();
        glow_paint.anti_alias = true;
        glow_paint.set_color(glow_color);

        self.canvas.stroke_path(
            &path,
            &glow_paint,
            &Stroke {
                width: self.config.oscilloscope_glow_thickness,
                miter_limit: 2.0,
                line_cap: LineCap::Butt,
                line_join: LineJoin::Bevel,
                dash: None
            },
            Transform::identity(),
            None
        );

        let mut line_paint = Paint::default();
        line_paint.anti_alias = true;
        line_paint.set_color(color);

        self.canvas.stroke_path(
            &path,
            &line_paint,
            &Stroke {
                width: self.config.oscilloscope_line_thickness,
                miter_limit: 1.0,
                line_cap: LineCap::Butt,
                line_join: LineJoin::Bevel,
                dash: None
            },
            Transform::identity(),
            None
        );
    }

    fn draw_oscilloscope_dividers(&mut self, pos: Rect, channel_width: f32) {
        let cache_valid = self.oscilloscope_divider_cache
            .as_ref()
            .map(|p| p.width() == channel_width as u32 && p.height() == pos.height() as u32)
            .unwrap_or(false);

        if !cache_valid {
            let mut divider_pixmap = Pixmap::new(channel_width as u32, pos.height() as u32).unwrap();

            let mut divider_paint = Paint::default();
            divider_paint.anti_alias = false;
            divider_paint.shader = LinearGradient::new(
                Point::from_xy(pos.x(), 0.0),
                Point::from_xy(pos.x() + (channel_width / 2.0), 0.0),
                vec![
                    GradientStop::new(0.0, self.config.divider_color),
                    GradientStop::new((2.0 * self.config.divider_width as f32) / channel_width, Color::TRANSPARENT),
                    GradientStop::new(1.0, Color::TRANSPARENT)
                ],
                SpreadMode::Reflect,
                Transform::identity()
            ).unwrap();

            divider_pixmap.fill_rect(
                Rect::from_xywh(0.0, 0.0, channel_width, pos.height()).unwrap(),
                &divider_paint,
                Transform::identity(),
                None
            );

            self.oscilloscope_divider_cache = Some(divider_pixmap);
        }

        for i in 0..(pos.width() / channel_width) as i32 {
            self.canvas.draw_pixmap(
                (pos.x() + (channel_width * i as f32)) as i32,
                pos.y() as i32,
                self.oscilloscope_divider_cache.as_ref().unwrap().as_ref(),
                &PixmapPaint::default(),
                Transform::identity(),
                None
            );
        }
    }

    pub fn draw_oscilloscopes(&mut self, pos: Rect, max_channels_per_row: usize) {
        self.canvas.fill_rect(
            pos,
            &Paint::default(),
            Transform::identity(),
            None
        );

        let channel_indices: Vec<usize> = (0..self.channels)
            .filter(|&i| !self.config.settings.settings(i).unwrap().hidden())
            .collect();

        for row in channel_indices.chunks(max_channels_per_row) {
            let channel_width = pos.width() / row.len() as f32;
            for &channel in row {
                let channel_pos = Rect::from_xywh(
                    pos.x() + (channel_width * channel as f32),
                    pos.y(),
                    channel_width,
                    pos.height()
                ).unwrap();
                self.draw_oscilloscope_view(channel, channel_pos);
            }
            self.draw_oscilloscope_dividers(pos, channel_width);
        }
    }
}
