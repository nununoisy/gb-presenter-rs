use anyhow::{Context, Result};
use serde::{Serialize, Deserialize};
use crate::visualizer::channel_settings::ChannelSettingsManager;

#[derive(Serialize, Deserialize, Clone)]
#[serde(default)]
pub struct PianoRollConfig {
    pub settings: ChannelSettingsManager,
    pub key_length: f32,
    pub key_thickness: f32,
    pub octave_count: u32,
    pub speed_multiplier: u32,
    pub starting_octave: i32,
    pub waveform_height: u32,
    pub oscilloscope_glow_thickness: f32,
    pub oscilloscope_line_thickness: f32
}

impl Default for PianoRollConfig {
    fn default() -> Self {
        Self {
            settings: ChannelSettingsManager::default(),
            key_length: 24.0,
            key_thickness: 5.0,
            octave_count: 9,
            speed_multiplier: 1,
            starting_octave: 0,
            waveform_height: 48,
            oscilloscope_glow_thickness: 2.0,
            oscilloscope_line_thickness: 0.75,
        }
    }
}

#[derive(Serialize, Deserialize, Default, Clone)]
#[serde(default)]
pub struct Config {
    pub piano_roll: PianoRollConfig
}

impl Config {
    pub fn from_toml(config: &str) -> Result<Self> {
        toml::from_str(config).context("Importing configuration")
    }

    pub fn export(&self) -> Result<String> {
        toml::to_string(&self).context("Exporting configuration")
    }
}
