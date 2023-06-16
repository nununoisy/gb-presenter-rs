use raqote::Color;
use sameboy::ApuChannel;
use super::ChannelState;

#[derive(Clone)]
pub struct ChannelSettings(String, bool, Vec<Color>);

impl ChannelSettings {
    pub fn new(name: &str, colors: &[Color]) -> Self {
        Self(name.to_string(), false, colors.to_vec())
    }

    pub fn name(&self) -> String {
        self.0.clone()
    }

    pub fn hidden(&self) -> bool {
        self.1
    }

    pub fn color(&self, state: &ChannelState) -> Option<Color> {
        let result = self.2.get(state.timbre).cloned();
        if let Some(color) = &result {
            if state.volume == 0 {
                return Some(Color::new(color.a(), color.r() / 2 + 0x10, color.g() / 2 + 0x10, color.b() / 2 + 0x10));
            }
        }
        result
    }

    pub fn num_colors(&self) -> usize {
        self.2.len()
    }
}

#[derive(Clone)]
pub struct ChannelSettingsManager {
    pulse1: ChannelSettings,
    pulse2: ChannelSettings,
    wave: ChannelSettings,
    noise: ChannelSettings
}

impl ChannelSettingsManager {
    pub fn new(pulse1: ChannelSettings, pulse2: ChannelSettings, wave: ChannelSettings, noise: ChannelSettings) -> Self {
        Self {
            pulse1,
            pulse2,
            wave,
            noise
        }
    }

    pub fn settings(&self, channel: ApuChannel) -> ChannelSettings {
        match channel {
            ApuChannel::Pulse1 => self.pulse1.clone(),
            ApuChannel::Pulse2 => self.pulse2.clone(),
            ApuChannel::Wave => self.wave.clone(),
            ApuChannel::Noise => self.noise.clone()
        }
    }

    pub fn settings_mut(&mut self, channel: ApuChannel) -> &mut ChannelSettings {
        match channel {
            ApuChannel::Pulse1 => &mut self.pulse1,
            ApuChannel::Pulse2 => &mut self.pulse2,
            ApuChannel::Wave => &mut self.wave,
            ApuChannel::Noise => &mut self.noise
        }
    }
}

impl Default for ChannelSettingsManager {
    fn default() -> Self {
        Self {
            pulse1: ChannelSettings::new("Pulse 1", &[
                Color::new(0xFF, 0xFF, 0xA0, 0xA0),
                Color::new(0xFF, 0xFF, 0x40, 0xFF),
                Color::new(0xFF, 0xFF, 0x40, 0x40),
                Color::new(0xFF, 0xFF, 0x40, 0xFF)
            ]),
            pulse2: ChannelSettings::new("Pulse 2", &[
                Color::new(0xFF, 0xFF, 0xE0, 0xA0),
                Color::new(0xFF, 0xFF, 0xC0, 0x40),
                Color::new(0xFF, 0xFF, 0xFF, 0x40),
                Color::new(0xFF, 0xFF, 0xC0, 0x40)
            ]),
            wave: ChannelSettings::new("Wave", &[
                Color::new(0xFF, 0x40, 0xFF, 0x40),
                Color::new(0xFF, 0x9A, 0x4F, 0xFF),
                Color::new(0xFF, 0x38, 0xAB, 0xF2),
                Color::new(0xFF, 0xAC, 0xED, 0x32),
                Color::new(0xFF, 0x24, 0x7B, 0xA0),
                Color::new(0xFF, 0x0F, 0xF4, 0xC6)
            ]),
            noise: ChannelSettings::new("Noise", &[
                Color::new(0xFF, 0xC0, 0xC0, 0xC0),
                Color::new(0xFF, 0x80, 0xF0, 0xFF)
            ])
        }
    }
}
