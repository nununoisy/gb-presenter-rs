use std::collections::{BTreeMap, HashMap};
use tiny_skia::Color;
use csscolorparser::Color as CssColor;
use serde::{Serialize, Serializer, Deserialize, Deserializer};
use super::ChannelState;

#[derive(Clone)]
pub struct ChannelSettings(String, String, bool, Vec<Color>);

impl ChannelSettings {
    pub fn new(chip: &str, name: &str, colors: &[Color]) -> Self {
        Self(chip.to_string(), name.to_string(), false, colors.to_vec())
    }

    pub fn chip(&self) -> String {
        self.0.clone()
    }

    pub fn name(&self) -> String {
        self.1.clone()
    }

    pub fn hidden(&self) -> bool {
        self.2
    }

    pub fn color(&self, state: &ChannelState) -> Option<Color> {
        let color_index = match self.3.len() {
            0 => state.timbre,
            max_index => state.timbre % max_index
        };

        let result = self.3.get(color_index).cloned();
        if let Some(color) = &result {
            if state.volume == 0.0 {
                return Some(Color::from_rgba(
                    color.red() / 2.0 + 0.0625,
                    color.green() / 2.0 + 0.0625,
                    color.blue() / 2.0 + 0.0625,
                    color.alpha()
                ).unwrap());
            }
        }
        result
    }

    pub fn colors(&self) -> Vec<Color> {
        self.3.clone()
    }

    pub fn num_colors(&self) -> usize {
        self.3.len()
    }

    pub fn set_hidden(&mut self, hidden: bool) {
        self.2 = hidden;
    }

    pub fn set_colors(&mut self, colors: &[Color]) {
        self.3 = colors.to_vec();
    }
}

impl Default for ChannelSettings {
    fn default() -> Self {
        Self::new("<?>", "<?>", &[Color::from_rgba8(0x90, 0x90, 0x90, 0xFF)])
    }
}

#[derive(Clone)]
pub struct ChannelSettingsManager(Vec<ChannelSettings>);

impl ChannelSettingsManager {
    pub fn settings(&self, channel: usize) -> Option<&ChannelSettings> {
        self.0.get(channel)
    }

    pub fn settings_mut(&mut self, channel: usize) -> Option<&mut ChannelSettings> {
        self.0.get_mut(channel)
    }

    pub fn settings_by_name(&self, chip: &str, channel: &str) -> Option<&ChannelSettings> {
        self.0
            .iter()
            .find(|settings| settings.chip().as_str() == chip && settings.name().as_str() == channel)
    }

    pub fn settings_mut_by_name(&mut self, chip: &str, channel: &str) -> Option<&mut ChannelSettings> {
        self.0
            .iter_mut()
            .find(|settings| settings.chip().as_str() == chip && settings.name().as_str() == channel)
    }

    pub fn to_map(&self) -> HashMap<(String, String), ChannelSettings> {
        let mut result: HashMap<(String, String), ChannelSettings> = HashMap::new();

        for settings in self.0.iter() {
            result.insert((settings.chip(), settings.name()), settings.clone());
        }

        result
    }

    pub fn apply_from_map(&mut self, map: &HashMap<(String, String), ChannelSettings>) {
        for ((chip, channel), settings) in map {
            match self.settings_mut_by_name(chip.as_str(), channel.as_str()) {
                Some(inner_settings) => {
                    debug_assert_eq!(inner_settings.chip(), chip.clone());
                    debug_assert_eq!(inner_settings.name(), channel.clone());

                    inner_settings.set_colors(&settings.colors());
                    inner_settings.set_hidden(settings.hidden());
                },
                None => {
                    unimplemented!()
                }
            }
        }
    }
}

impl Default for ChannelSettingsManager {
    fn default() -> Self {
        Self(vec![
            ChannelSettings::new("LR35902", "Pulse 1", &[
                Color::from_rgba8(0xFF, 0xBF, 0xD4, 0xFF),
                Color::from_rgba8(0xFF, 0x73, 0x8A, 0xFF),
                Color::from_rgba8(0xFF, 0x40, 0x40, 0xFF),
                Color::from_rgba8(0xFF, 0x73, 0x8A, 0xFF)
            ]),
            ChannelSettings::new("LR35902", "Pulse 2", &[
                Color::from_rgba8(0xFF, 0xE0, 0xA0, 0xFF),
                Color::from_rgba8(0xFF, 0xC0, 0x40, 0xFF),
                Color::from_rgba8(0xFF, 0xFF, 0x40, 0xFF),
                Color::from_rgba8(0xFF, 0xC0, 0x40, 0xFF)
            ]),
            ChannelSettings::new("LR35902", "Wave", &[
                Color::from_rgba8(0x40, 0xFF, 0x40, 0xFF),
                Color::from_rgba8(0x9A, 0x4F, 0xFF, 0xFF),
                Color::from_rgba8(0x38, 0xAB, 0xF2, 0xFF),
                Color::from_rgba8(0xAC, 0xED, 0x32, 0xFF),
                Color::from_rgba8(0x24, 0x7B, 0xA0, 0xFF),
                Color::from_rgba8(0x0F, 0xF4, 0xC6, 0xFF)
            ]),
            ChannelSettings::new("LR35902", "Noise", &[
                Color::from_rgba8(0xC0, 0xC0, 0xC0, 0xFF),
                Color::from_rgba8(0x80, 0xF0, 0xFF, 0xFF)
            ]),
            ChannelSettings::new("LR35902 (2x)", "Pulse 1", &[
                Color::from_rgba8(0xB5, 0xE1, 0xFF, 0xFF),
                Color::from_rgba8(0x56, 0xC8, 0xFF, 0xFF),
                Color::from_rgba8(0x0E, 0x80, 0xC8, 0xFF),
                Color::from_rgba8(0x56, 0xC8, 0xFF, 0xFF)
            ]),
            ChannelSettings::new("LR35902 (2x)", "Pulse 2", &[
                Color::from_rgba8(0xDB, 0x95, 0xB8, 0xFF),
                Color::from_rgba8(0xB3, 0x56, 0x84, 0xFF),
                Color::from_rgba8(0x8A, 0x25, 0x57, 0xFF),
                Color::from_rgba8(0xB3, 0x56, 0x84, 0xFF)
            ]),
            ChannelSettings::new("LR35902 (2x)", "Wave", &[
                Color::from_rgba8(0xFF, 0x99, 0xEE, 0xFF),
                Color::from_rgba8(0xD8, 0xE1, 0xEB, 0xFF),
                Color::from_rgba8(0x69, 0x8D, 0xF0, 0xFF),
                Color::from_rgba8(0xFA, 0xA2, 0x43, 0xFF),
                Color::from_rgba8(0x1B, 0xE3, 0x93, 0xFF),
                Color::from_rgba8(0x37, 0xCB, 0xF0, 0xFF)
            ]),
            ChannelSettings::new("LR35902 (2x)", "Noise", &[
                Color::from_rgba8(0x07, 0x7D, 0x5A, 0xFF),
                Color::from_rgba8(0x9F, 0xB8, 0xED, 0xFF)
            ])
        ])
    }
}

#[derive(Serialize, Deserialize, Default, Clone)]
#[serde(default)]
struct PianoRollChannelConfig {
    pub hidden: bool,
    #[serde(flatten)]
    pub colors: BTreeMap<String, CssColor>
}

impl Serialize for ChannelSettingsManager {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut settings: BTreeMap<String, BTreeMap<String, PianoRollChannelConfig>> = BTreeMap::new();
        for channel_settings in self.0.iter() {
            let config = PianoRollChannelConfig {
                hidden: channel_settings.hidden(),
                colors: BTreeMap::from_iter(
                    channel_settings.colors()
                        .iter()
                        .enumerate()
                        .filter_map(|(i, c)| {
                            let css_color = CssColor::new(c.red() as _, c.green() as _, c.blue() as _, c.alpha() as _);
                            match channel_settings.name().as_str() {
                                "Pulse 1" | "Pulse 2" => (i < 4).then(|| (format!("duty{:X}", i), css_color)),
                                "Wave" => (i < 6).then(|| (format!("wave{:X}", i), css_color)),
                                "Noise" => (i < 2).then(|| (format!("mode{:X}", i), css_color)),
                                _ => None
                            }
                        })
                )
            };

            settings.entry(channel_settings.chip())
                .or_insert(BTreeMap::new())
                .insert(channel_settings.name(), config);
        }

        settings.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for ChannelSettingsManager {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let mut result = Self::default();
        let settings: BTreeMap<String, BTreeMap<String, PianoRollChannelConfig>> = BTreeMap::deserialize(deserializer)?;

        for (chip, chip_settings) in settings {
            for (channel, channel_settings) in chip_settings {
                if let Some(settings) = result.settings_mut_by_name(&chip, &channel) {
                    let mut colors = settings.colors();
                    for (color_key, css_color) in channel_settings.colors.iter() {
                        let index = match (channel.as_str(), color_key.as_str()) {
                            ("Pulse 1" | "Pulse 2", "duty0") => 0,
                            ("Pulse 1" | "Pulse 2", "duty1") => 1,
                            ("Pulse 1" | "Pulse 2", "duty2") => 2,
                            ("Pulse 1" | "Pulse 2", "duty3") => 3,
                            ("Wave", "wave0") => 0,
                            ("Wave", "wave1") => 1,
                            ("Wave", "wave2") => 2,
                            ("Wave", "wave3") => 3,
                            ("Wave", "wave4") => 4,
                            ("Wave", "wave5") => 5,
                            ("Noise", "mode0") => 0,
                            ("Noise", "mode1") => 1,
                            _ => continue
                        };
                        colors.get_mut(index).map(|c| {
                            c.set_red(css_color.r as _);
                            c.set_green(css_color.g as _);
                            c.set_blue(css_color.b as _);
                            c.set_alpha(1.0);
                        });
                    }
                    settings.set_colors(&colors);
                    settings.set_hidden(channel_settings.hidden);
                }
            }
        }

        Ok(result)
    }
}