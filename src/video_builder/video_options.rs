use std::collections::HashMap;
use ffmpeg_next::Rational;

#[derive(Clone)]
pub struct VideoOptions {
    pub output_path: String,
    pub metadata: HashMap<String, String>,
    pub background_path: Option<String>,

    pub video_time_base: Rational,
    pub video_codec: String,
    pub video_codec_params: HashMap<String, String>,
    pub pixel_format_in: String,
    pub pixel_format_out: String,
    pub resolution_in: (u32, u32),
    pub resolution_out: (u32, u32),

    pub audio_time_base: Rational,
    pub audio_codec: String,
    pub audio_codec_params: HashMap<String, String>,
    pub audio_channels: i32,
    pub sample_format_in: String,
    pub sample_format_out: String,
    pub sample_rate: i32
}
