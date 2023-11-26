use anyhow::bail;
use std::str::FromStr;
use std::ffi::OsStr;
use sameboy::{Model, Revision};
use crate::config::Config;
use crate::video_builder::video_options::VideoOptions;

pub const FRAME_RATE: i32 = 60;

macro_rules! extra_str_traits {
    ($t: ty) => {
        impl From<&OsStr> for $t {
            fn from(value: &OsStr) -> Self {
                <$t>::from_str(value.to_str().unwrap()).unwrap()
            }
        }

        impl From<String> for $t {
            fn from(value: String) -> Self {
                <$t>::from_str(value.as_str()).unwrap()
            }
        }
    }
}

#[derive(Copy, Clone)]
pub enum StopCondition {
    Frames(u64),
    Loops(usize)
}

impl FromStr for StopCondition {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let parts: Vec<_> = s.split(':').collect();
        if parts.len() != 2 {
            bail!("Stop condition format invalid, try one of 'time:3', 'time:nsfe', 'frames:180', or 'loops:2'.");
        }

        match parts[0] {
            "time" => {
                let time = u64::from_str(parts[1])?;
                Ok(StopCondition::Frames(time * FRAME_RATE as u64))
            },
            "frames" => {
                let frames = u64::from_str(parts[1])?;
                Ok(StopCondition::Frames(frames))
            },
            "loops" => {
                let loops = usize::from_str(parts[1])?;
                Ok(StopCondition::Loops(loops))
            },
            _ => bail!("Unknown condition type {}. Valid types are 'time', 'frames', and 'loops'", parts[0])
        }
    }
}

extra_str_traits!(StopCondition);

#[derive(Clone)]
pub enum RenderInput {
    None,
    GBS(String),
    LSDj(String, String),
    LSDj2x(String, String, String, String),
    VGM(String)
}

#[derive(Clone)]
pub struct RendererOptions {
    pub input: RenderInput,
    pub video_options: VideoOptions,

    pub track_index: u8,
    pub track_index_2x: u8,
    pub stop_condition: StopCondition,
    pub fadeout_length: u64,
    pub auto_lsdj_sync: bool,

    pub model: Model,
    pub config: Config
}

impl Default for RendererOptions {
    fn default() -> Self {
        Self {
            input: RenderInput::None,
            video_options: VideoOptions {
                output_path: "".to_string(),
                metadata: Default::default(),
                background_path: None,
                video_time_base: (70_224, 4_194_304).into(),
                video_codec: "libx264".to_string(),
                video_codec_params: Default::default(),
                pixel_format_in: "rgba".to_string(),
                pixel_format_out: "yuv420p".to_string(),
                resolution_in: (960, 540),
                resolution_out: (1920, 1080),
                audio_time_base: (1, 44_100).into(),
                audio_codec: "aac".to_string(),
                audio_codec_params: Default::default(),
                audio_channels: 2,
                sample_format_in: "s16".to_string(),
                sample_format_out: "fltp".to_string(),
                sample_rate: 44_100,
            },
            track_index: 0,
            track_index_2x: 0,
            stop_condition: StopCondition::Frames(300 * FRAME_RATE as u64),
            fadeout_length: 180,
            auto_lsdj_sync: false,
            model: Model::DMG(Revision::RevB),
            config: Config::default()
        }
    }
}

impl RendererOptions {
    pub fn set_resolution_smart(&mut self, w: u32, h: u32) {
        self.video_options.resolution_out = (w, h);

        self.video_options.resolution_in = if w >= h {
            (960, ((960.0 / w as f32) * (h as f32)) as u32)
        } else {
            (((960.0 / h as f32) * (w as f32)) as u32, 960)
        };

        println!("{}x{}", self.video_options.resolution_in.0, self.video_options.resolution_in.1);
    }
}
