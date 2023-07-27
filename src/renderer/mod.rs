pub mod render_options;
pub mod lsdj;
pub mod gbs;
pub mod vgm;

use std::cell::RefCell;
use std::fmt::{Display, Formatter};
use std::fs;
use std::rc::Rc;
use std::time::{Duration, Instant};
use ringbuf::{HeapRb, Rb};
use ringbuf::ring_buffer::RbBase;
use render_options::{RendererOptions, RenderInput};
use sameboy::{ApuChannel, Gameboy, JoypadButton, Model};
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
    gb: Box<Gameboy>,
    viz: Rc<RefCell<Visualizer>>,
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
    pub fn new(options: RendererOptions) -> Result<Self, String> {
        let gb = Gameboy::new(options.clone().model);
        let viz = Rc::new(RefCell::new(Visualizer::new()));
        let vb = VideoBuilder::new(options.video_options.clone())?;

        Ok(Self {
            options: options.clone(),
            gb,
            viz,
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

    pub fn start_encoding(&mut self) -> Result<(), String> {
        self.gb.set_sample_rate(self.options.video_options.sample_rate as usize);
        self.gb.emulate_joypad_bouncing(false);

        match &self.options.input {
            RenderInput::None => return Err("No input specified.".to_string()),
            RenderInput::GBS(gbs_path) => {
                let gbs = fs::read(gbs_path)
                    .map_err(|e| format!("Failed to read GBS! {}", e))?;
                self.gb.load_gbs(&gbs);
                self.gb.gbs_change_track(self.options.track_index);
            },
            RenderInput::LSDj(rom_path, sav_path) => {
                let boot_rom = fs::read(self.gb.preferred_boot_rom())
                    .map_err(|e| format!("Failed to read boot ROM! {}", e))?;
                self.gb.load_boot_rom(&boot_rom);

                let rom = fs::read(rom_path)
                    .map_err(|e| format!("Failed to read LSDj ROM! {}", e))?;
                self.gb.load_rom(&rom);

                let sav = fs::read(sav_path)
                    .map_err(|e| format!("Failed to read LSDj SAV! {}", e))?;
                self.gb.load_sram(&sav);

                println!("{}", self.gb.game_title());

                while !self.gb.boot_rom_finished() {
                    self.gb.run();
                }

                self.gb.joypad_macro_press(&[], Some(Duration::from_millis(5000)));
                lsdj::select_track_joypad_macro(&mut self.gb, self.options.track_index);
            },
            RenderInput::VGM(vgm_path) => {
                let vgm_data = fs::read(vgm_path)
                    .map_err(|e| format!("Failed to read VGM! {}", e))?;

                let mut vgm_s = vgm::Vgm::new(&vgm_data)?;
                let (gbs, engine_data) = vgm::converter::vgm_to_gbs(&mut vgm_s)?;

                println!("{:?}", engine_data.loop_data);

                self.gb.load_gbs(&gbs);
            }
        }

        self.gb.set_apu_receiver(Some(self.viz.clone()));

        match &self.options.input {
            RenderInput::LSDj(_, _) => self.gb.joypad_macro_press(&[JoypadButton::Start], None),
            _ => ()
        }
        // Clear the sample buffer to get rid of boot ROM ding and LSDj selection frame silence
        let _ = self.gb.get_audio_samples(None).unwrap();

        for ((chip, channel), settings) in &self.options.channel_settings {
            let mut viz = self.viz.borrow_mut();
            let manager = match chip.as_str() {
                "LR35902" => viz.settings_manager_mut(),
                _ => continue
            };
            let current_settings = manager.settings_mut(match channel.as_str() {
                "Pulse 1" => ApuChannel::Pulse1,
                "Pulse 2" => ApuChannel::Pulse2,
                "Wave" => ApuChannel::Wave,
                "Noise" => ApuChannel::Noise,
                _ => continue
            });
            current_settings.set_hidden(settings.hidden());
            current_settings.set_colors(&settings.colors());
        }

        self.vb.start_encoding()?;
        self.encode_start = Instant::now();

        Ok(())
    }

    pub fn step(&mut self) -> Result<bool, String> {
        self.gb.run_frame();

        self.viz.borrow_mut().clear();
        self.viz.borrow_mut().draw_oscilloscopes();
        self.viz.borrow_mut().draw_piano_roll();

        self.vb.push_video_data(&self.viz.borrow().get_canvas_buffer())?;
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

    pub fn finish_encoding(&mut self) -> Result<(), String> {
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
            RenderInput::LSDj(_, _) => lsdj::get_song_position(&mut self.gb),
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
                let remaining_secs = remaining_frames as f64 / 60.0;
                Some(Duration::from_secs_f64(self.elapsed().as_secs_f64() + remaining_secs))
            },
            None => None
        }
    }
}
