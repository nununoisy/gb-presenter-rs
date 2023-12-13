pub mod render_options;
pub mod lsdj;
pub mod gbs;
pub mod vgm;
pub mod m3u_searcher;

use anyhow::{Result, anyhow, bail};
use std::fmt::{Display, Formatter};
use std::fs;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use ringbuf::{HeapRb, Rb, ring_buffer::RbBase};
use render_options::{RendererOptions, RenderInput};
use sameboy::{Gameboy, JoypadButton};
use crate::renderer::render_options::StopCondition;
use crate::video_builder;
use crate::video_builder::VideoBuilder;
use crate::visualizer::Visualizer;

#[derive(Copy, Clone, Default, PartialEq)]
pub struct SongPosition {
    pub row: u8,
    pub end: bool
}

impl Display for SongPosition {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        if self.end {
            write!(f, "end")
        } else {
            write!(f, "{}", self.row)
        }
    }
}

pub struct Renderer {
    options: RendererOptions,
    gb: Gameboy,
    gb_2x: Gameboy,
    viz: Arc<Mutex<Visualizer>>,
    end_detector: Arc<Mutex<lsdj::EndDetector>>,
    vgm_2x: bool,
    vb: VideoBuilder,

    cur_frame: u64,
    encode_start: Instant,
    frame_timestamp: f64,
    frame_times: HeapRb<f64>,
    last_position: SongPosition,
    loop_count: u64,
    loop_duration: Option<u64>,
    fadeout_timer: Option<u64>,
    expected_duration: Option<usize>
}

impl Renderer {
    pub fn new(options: RendererOptions) -> Result<Self> {
        let gb = Gameboy::new(0, options.clone().model)?;
        let gb_2x = Gameboy::new(1, options.clone().model)?;
        let viz = Arc::new(Mutex::new(Visualizer::new(
            8,
            options.video_options.resolution_in.0,
            options.video_options.resolution_in.1,
            options.video_options.sample_rate as u32,
            options.config.clone().piano_roll
        )));
        let vb = VideoBuilder::new(options.video_options.clone())?;
        let end_detector = Arc::new(Mutex::new(lsdj::EndDetector::new()));

        Ok(Self {
            options: options.clone(),
            gb,
            gb_2x,
            viz,
            end_detector,
            vgm_2x: false,
            vb,
            cur_frame: 0,
            encode_start: Instant::now(),
            frame_timestamp: 0.0,
            frame_times: HeapRb::new(600),
            last_position: SongPosition::default(),
            loop_count: 0,
            loop_duration: None,
            fadeout_timer: None,
            expected_duration: None
        })
    }

    pub fn is_2x(&self) -> bool {
        match &self.options.input {
            RenderInput::LSDj2x(_, _, _, _) => true,
            RenderInput::VGM(_, _, _) => self.vgm_2x,
            _ => false
        }
    }

    pub fn start_encoding(&mut self) -> Result<()> {
        self.gb.set_sample_rate(self.options.video_options.sample_rate as usize);
        self.gb.emulate_joypad_bouncing(false);
        self.gb.allow_illegal_inputs(true);
        self.gb.set_rendering_disabled(true);

        self.vgm_2x = false;

        match &self.options.input {
            RenderInput::None => bail!("No input specified."),
            RenderInput::GBS(gbs_path) => {
                let gbs = fs::read(gbs_path)
                    .map_err(|e| anyhow!("Failed to read GBS! {}", e))?;
                self.gb.load_gbs(&gbs)
                    .map_err(|e| anyhow!("Failed to load GBS! {}", e))?;
                self.gb.gbs_change_track(self.options.track_index);
            },
            RenderInput::LSDj(rom_path, sav_path) => {
                let rom = fs::read(rom_path)
                    .map_err(|e| anyhow!("Failed to read LSDj ROM! {}", e))?;
                self.gb.load_rom(&rom);

                let sav = fs::read(sav_path)
                    .map_err(|e| anyhow!("Failed to read LSDj SAV! {}", e))?;
                self.gb.load_sram(&sav);

                println!("{} {}", self.gb.game_title().unwrap_or("<error>".to_string()), self.options.track_index);

                while !self.gb.boot_rom_finished() {
                    self.gb.run();
                }

                let sync_role = if self.options.auto_lsdj_sync {
                    lsdj::SyncRole::NoSync
                } else {
                    lsdj::SyncRole::Ignore
                };

                self.gb.joypad_macro_press(&[], Some(Duration::from_secs(5)));
                lsdj::select_track_joypad_macro(&mut self.gb, self.options.track_index, sync_role);

                self.gb.set_memory_interceptor(Some(self.end_detector.clone()));
            },
            RenderInput::LSDj2x(rom_path, sav_path, rom_path_2x, sav_path_2x) => {
                self.gb_2x.set_sample_rate(self.options.video_options.sample_rate as usize);
                self.gb_2x.emulate_joypad_bouncing(false);
                self.gb_2x.allow_illegal_inputs(true);
                self.gb_2x.set_rendering_disabled(true);

                let rom = fs::read(rom_path)
                    .map_err(|e| anyhow!("Failed to read LSDj ROM! {}", e))?;
                self.gb.load_rom(&rom);

                let sav = fs::read(sav_path)
                    .map_err(|e| anyhow!("Failed to read LSDj SAV! {}", e))?;
                self.gb.load_sram(&sav);

                let rom_2x = fs::read(rom_path_2x)
                    .map_err(|e| anyhow!("Failed to read LSDj ROM! {}", e))?;
                self.gb_2x.load_rom(&rom_2x);

                let sav_2x = fs::read(sav_path_2x)
                    .map_err(|e| anyhow!("Failed to read LSDj SAV! {}", e))?;
                self.gb_2x.load_sram(&sav_2x);

                println!("(1) {} {}", self.gb.game_title().unwrap_or("<error>".to_string()), self.options.track_index);
                println!("(2) {} {}", self.gb_2x.game_title().unwrap_or("<error>".to_string()), self.options.track_index_2x);

                while !self.gb.boot_rom_finished() {
                    self.gb.run();
                }
                while !self.gb_2x.boot_rom_finished() {
                    self.gb_2x.run();
                }

                let (sync_role, sync_role_2x) = if self.options.auto_lsdj_sync {
                    (lsdj::SyncRole::Primary, lsdj::SyncRole::Secondary)
                } else {
                    (lsdj::SyncRole::Ignore, lsdj::SyncRole::Ignore)
                };

                self.gb.joypad_macro_press(&[], Some(Duration::from_secs(5)));
                lsdj::select_track_joypad_macro(&mut self.gb, self.options.track_index, sync_role);
                self.gb_2x.joypad_macro_press(&[], Some(Duration::from_secs(5)));
                lsdj::select_track_joypad_macro(&mut self.gb_2x, self.options.track_index_2x, sync_role_2x);

                self.gb.set_memory_interceptor(Some(self.end_detector.clone()));
            }
            RenderInput::VGM(vgm_path, engine_rate, tma_offset) => {
                let vgm_data = fs::read(vgm_path)
                    .map_err(|e| anyhow!("Failed to read VGM! {}", e))?;

                let mut vgm_s = vgm::Vgm::new(&vgm_data)?;
                self.vgm_2x = vgm_s.lr35902_clock().map(|(_c, v)| v).unwrap_or_default();

                let gbs = vgm::converter::vgm_to_gbs(&mut vgm_s, false, *engine_rate, *tma_offset)?;
                self.gb.load_gbs(&gbs)
                    .map_err(|e| anyhow!("Failed to convert VGM to valid GBS! {}", e))?;

                if self.is_2x() {
                    self.gb_2x.set_sample_rate(self.options.video_options.sample_rate as usize);
                    self.gb_2x.emulate_joypad_bouncing(false);
                    self.gb_2x.allow_illegal_inputs(true);
                    self.gb_2x.set_rendering_disabled(true);

                    let gbs_2x = vgm::converter::vgm_to_gbs(&mut vgm_s, true, *engine_rate, *tma_offset)?;
                    self.gb_2x.load_gbs(&gbs_2x)
                        .map_err(|e| anyhow!("Failed to convert VGM to valid GBS! {}", e))?;
                }
            }
        }

        self.gb.joypad_release_all();
        self.gb.set_apu_receiver(Some(self.viz.clone()));
        // Clear the sample buffer to get rid of boot ROM ding and LSDj selection frame silence
        let _ = self.gb.get_audio_samples(None).unwrap();

        if self.is_2x() {
            self.gb_2x.joypad_release_all();
            self.gb_2x.set_apu_receiver(Some(self.viz.clone()));
            let _ = self.gb_2x.get_audio_samples(None).unwrap();

            if matches!(&self.options.input, RenderInput::LSDj2x(_, _, _, _)) {
                self.gb.run_frame();
                self.gb_2x.run_frame();
                self.gb.connect_console(&mut self.gb_2x);
            }
        }

        {
            let mut viz = self.viz.lock().unwrap();

            let all_channels_hidden = (0..8).all(|i| viz.settings_manager().settings(i).unwrap().hidden());
            if all_channels_hidden {
                bail!("At least one channel must be visible!");
            }

            if !self.is_2x() {
                for channel in 4..8 {
                    viz.settings_manager_mut()
                        .settings_mut(channel)
                        .unwrap()
                        .set_hidden(true);
                }
            }
        }

        self.end_detector.lock().unwrap().reset();

        self.vb.start_encoding()?;
        self.encode_start = Instant::now();
        self.frame_timestamp = 0.0;
        self.frame_times.clear();
        self.last_position = SongPosition::default();
        self.loop_count = 0;
        self.loop_duration = None;
        self.fadeout_timer = None;
        self.expected_duration = None;

        Ok(())
    }

    pub fn step(&mut self) -> Result<bool> {
        if self.is_2x() {
            self.gb.run_frame_sync(&mut self.gb_2x);

            if self.frame_timestamp < 0.5 && matches!(&self.options.input, RenderInput::LSDj2x(_, _, _, _)) {
                self.gb.set_joypad_button(JoypadButton::Start, true);
            } else {
                self.gb.joypad_release_all();
            }
            self.gb_2x.joypad_release_all();
        } else {
            self.gb.run_frame();

            if self.frame_timestamp < 0.5 && matches!(&self.options.input, RenderInput::LSDj(_, _)) {
                self.gb.set_joypad_button(JoypadButton::Start, true);
            } else {
                self.gb.joypad_release_all();
            }
        }

        {
            let mut viz = self.viz.lock().unwrap();
            viz.draw();
            self.vb.push_video_data(viz.get_canvas_buffer())?;
        }

        if self.is_2x() {
            if let Some(audio) = self.gb.get_audio_samples(Some(self.vb.audio_frame_size())) {
                if let Some(audio_2x) = self.gb_2x.get_audio_samples(Some(self.vb.audio_frame_size())) {
                    let adjusted_audio: Vec<i16> = match self.fadeout_timer {
                        Some(t) => {
                            let volume_divisor = (self.options.fadeout_length as f64 / t as f64) as i16;
                            std::iter::zip(audio, audio_2x)
                                .map(|(s, s_2x)| s.saturating_add(s_2x) / (2 * volume_divisor))
                                .collect()
                        },
                        None => {
                            std::iter::zip(audio, audio_2x)
                                .map(|(s, s_2x)| s.saturating_add(s_2x) / 2)
                                .collect()
                        }
                    };
                    self.vb.push_audio_data(video_builder::as_u8_slice(&adjusted_audio))?;
                }
            }
        } else {
            if let Some(audio) = self.gb.get_audio_samples(Some(self.vb.audio_frame_size())) {
                let adjusted_audio = match self.fadeout_timer {
                    Some(t) => {
                        let volume_divisor = (self.options.fadeout_length as f64 / t as f64) as i16;
                        audio.iter().map(|s| s / volume_divisor).collect()
                    },
                    None => audio
                };
                self.vb.push_audio_data(video_builder::as_u8_slice(&adjusted_audio))?;
            }
        }

        self.vb.step_encoding()?;

        let elapsed_secs = self.elapsed().as_secs_f64();
        let frame_time = elapsed_secs - self.frame_timestamp;
        self.frame_timestamp = elapsed_secs;

        self.frame_times.push_overwrite(frame_time);

        self.expected_duration = self.next_expected_duration();
        self.fadeout_timer = self.next_fadeout_timer();

        if let Some(t) = self.fadeout_timer {
            if t == 0 {
                return Ok(false)
            }
        }

        self.cur_frame += 1;

        if let Some(current_position) = self.song_position() {
            if current_position.row < self.last_position.row {
                self.loop_count += 1;
                if self.loop_duration.is_none() {
                    self.loop_duration = Some(self.cur_frame);
                }
            }
            self.last_position = current_position;
        }

        Ok(true)
    }

    pub fn finish_encoding(&mut self) -> Result<()> {
        self.vb.finish_encoding()?;

        Ok(())
    }

    pub fn current_frame(&self) -> u64 {
        self.cur_frame
    }

    pub fn elapsed(&self) -> Duration {
        self.encode_start.elapsed()
    }

    fn next_expected_duration(&self) -> Option<usize> {
        if self.expected_duration.is_some() {
            return self.expected_duration;
        }

        match self.options.stop_condition {
            StopCondition::Frames(stop_frames) => Some((stop_frames + self.options.fadeout_length) as usize),
            StopCondition::Loops(stop_loop_count) => {
                match self.loop_duration {
                    Some(d) => Some(self.options.fadeout_length as usize + d as usize * stop_loop_count),
                    None => None
                }
            }
        }
    }

    fn next_fadeout_timer(&self) -> Option<u64> {
        match self.fadeout_timer {
            Some(0) => Some(0),
            Some(t) => Some(t - 1),
            None => {
                if self.last_position.end {
                    return Some(self.options.fadeout_length);
                }

                match self.options.stop_condition {
                    StopCondition::Loops(stop_loop_count) => {
                        if self.loop_count >= stop_loop_count as u64 {
                            Some(self.options.fadeout_length)
                        } else {
                            None
                        }
                    },
                    StopCondition::Frames(stop_frames) => {
                        if self.current_frame() >= stop_frames {
                            Some(self.options.fadeout_length)
                        } else {
                            None
                        }
                    }
                }
            }
        }
    }

    pub fn song_position(&mut self) -> Option<SongPosition> {
        match &self.options.input {
            RenderInput::LSDj(_, _) => lsdj::get_song_position(&mut self.gb, &self.end_detector),
            RenderInput::LSDj2x(_, _, _, _) => lsdj::get_song_position(&mut self.gb, &self.end_detector),
            _ => None
        }
    }

    pub fn loop_count(&self) -> u64 {
        self.loop_count
    }

    pub fn instantaneous_fps(&self) -> u32 {
        match self.frame_times.iter().last().cloned() {
            Some(ft) => (1.0 / ft) as u32,
            None => 0
        }
    }

    pub fn average_fps(&self) -> u32 {
        if self.frame_times.is_empty() {
            return 0;
        }
        (self.frame_times.len() as f64 / self.frame_times.iter().sum::<f64>()) as u32
    }

    pub fn encode_rate(&self) -> f64 {
        self.average_fps() as f64 / 60.0
    }

    pub fn encoded_duration(&self) -> Duration {
        self.vb.encoded_video_duration()
    }

    pub fn encoded_size(&self) -> usize {
        self.vb.encoded_video_size()
    }

    pub fn expected_duration_frames(&self) -> Option<usize> {
        self.expected_duration
    }

    pub fn expected_duration(&self) -> Option<Duration> {
        match self.expected_duration {
            Some(d) => {
                let secs = d as f64 / 60.0;
                Some(Duration::from_secs_f64(secs))
            },
            None => None
        }
    }

    pub fn eta_duration(&self) -> Option<Duration> {
        match self.expected_duration {
            Some(expected_duration) => {
                let remaining_frames = expected_duration - self.current_frame() as usize;
                let average_fps = u32::max(self.average_fps(), 1) as f64;
                let remaining_secs = remaining_frames as f64 / average_fps;
                Some(Duration::from_secs_f64(self.elapsed().as_secs_f64() + remaining_secs))
            },
            None => None
        }
    }
}
