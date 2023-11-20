use std::collections::hash_map::DefaultHasher;
use std::hash::Hasher;
use std::sync::{Arc, Mutex};
use sameboy_sys::{GB_gameboy_t, GB_sample_t, GB_channel_t, GB_channel_t_GB_NOISE, GB_channel_t_GB_SQUARE_1, GB_channel_t_GB_SQUARE_2, GB_channel_t_GB_WAVE, GB_apu_set_sample_callback, GB_get_apu_wave_table, GB_get_channel_amplitude, GB_get_channel_edge_triggered, GB_get_channel_period, GB_get_channel_volume, GB_is_channel_muted, GB_set_channel_muted, GB_get_sample_rate, GB_set_sample_rate, GB_set_highpass_filter_mode, GB_highpass_mode_t, GB_highpass_mode_t_GB_HIGHPASS_OFF, GB_highpass_mode_t_GB_HIGHPASS_ACCURATE, GB_highpass_mode_t_GB_HIGHPASS_REMOVE_DC_OFFSET, GB_set_interference_volume};
use super::Gameboy;

pub const AUDIO_BUFFER_INITIAL_SIZE: usize = 4 * 1024 * 1024;

// list(sorted(set(float(max(r, 0.5) * (2**s)) for r in range(0,8) for s in range(0,16))))
const NOISE_PERIODS: [f64; 68] = [
    0.5, 1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 10.0, 12.0, 14.0, 16.0, 20.0, 24.0, 28.0, 32.0, 40.0, 48.0, 56.0,
    64.0, 80.0, 96.0, 112.0, 128.0, 160.0, 192.0, 224.0, 256.0, 320.0, 384.0, 448.0, 512.0, 640.0, 768.0, 896.0, 1024.0,
    1280.0, 1536.0, 1792.0, 2048.0, 2560.0, 3072.0, 3584.0, 4096.0, 5120.0, 6144.0, 7168.0, 8192.0, 10240.0, 12288.0,
    14336.0, 16384.0, 20480.0, 24576.0, 28672.0, 32768.0, 40960.0, 49152.0, 57344.0, 65536.0, 81920.0, 98304.0, 114688.0,
    131072.0, 163840.0, 196608.0, 229376.0
];
const C_0: f64 = 16.351597831287;

#[derive(Copy, Clone, PartialEq, Debug, Default)]
pub enum ApuChannel {
    #[default]
    Pulse1,
    Pulse2,
    Wave,
    Noise
}

impl From<GB_channel_t> for ApuChannel {
    fn from(value: GB_channel_t) -> Self {
        match value {
            GB_channel_t_GB_SQUARE_1 => Self::Pulse1,
            GB_channel_t_GB_SQUARE_2 => Self::Pulse2,
            GB_channel_t_GB_WAVE => Self::Wave,
            GB_channel_t_GB_NOISE => Self::Noise,
            _ => unreachable!("Invalid GB_channel_t value")
        }
    }
}

impl From<ApuChannel> for GB_channel_t {
    fn from(value: ApuChannel) -> Self {
        match value {
            ApuChannel::Pulse1 => GB_channel_t_GB_SQUARE_1,
            ApuChannel::Pulse2 => GB_channel_t_GB_SQUARE_2,
            ApuChannel::Wave => GB_channel_t_GB_WAVE,
            ApuChannel::Noise => GB_channel_t_GB_NOISE
        }
    }
}

pub trait ApuStateReceiver {
    /// Receive an APU channel's state for visualization:
    /// - Console's ID (for multichip visualizations, e.g. 2xLSDj)
    /// - Amplitude (0-15)
    /// - Frequency (Hz)
    /// - Timbre (arbitrary index, e.g. for selecting a color)
    /// - Balance (0.0-1.0, 0.0=left, 0.5=center, 1.0=right)
    /// - Edge (if the scope should try to align to this point in time)
    fn receive(&mut self, id: usize, channel: ApuChannel, volume: u8, amplitude: u8, frequency: f64, timbre: usize, balance: f64, edge: bool);
}

fn send_pulse_channel_state(gb: &mut Gameboy, pulse2: bool, io_registers: &[u8]) {
    let io_base = if pulse2 { 0x15 } else { 0x10 };
    let nrx1 = io_registers[io_base + 1];
    let nr51 = io_registers[0x25];

    let id = unsafe { (*gb.inner()).id };

    let channel = if pulse2 { ApuChannel::Pulse2 } else { ApuChannel::Pulse1 };

    let mut volume: u8;
    let mut amplitude: u8;
    let period: u16;
    let edge: bool;
    unsafe {
        volume = GB_get_channel_volume(gb.as_mut_ptr(), channel.into());
        amplitude = GB_get_channel_amplitude(gb.as_mut_ptr(), channel.into());
        period = GB_get_channel_period(gb.as_mut_ptr(), channel.into());
        edge = GB_get_channel_edge_triggered(gb.as_mut_ptr(), channel.into());
    }

    let frequency = 131072.0 / (2048.0 - (period as f64));

    let timbre = (nrx1 >> 6) as usize;

    let mix_mask = if pulse2 { 0x22 } else { 0x11 };
    let mix = nr51 & mix_mask;
    let mix_l = (mix & 0xF0) != 0;
    let mix_r = (mix & 0x0F) != 0;

    let balance = match (mix_l, mix_r) {
        (false, false) => {
            amplitude = 0;
            volume = 0;
            0.5
        },
        (true, false) => 0.0,
        (false, true) => 1.0,
        (true, true) => 0.5
    };

    unsafe {
        (*gb.inner_mut()).apu_receiver
            .clone()
            .unwrap()
            .lock()
            .unwrap()
            .receive(id, channel, volume, amplitude, frequency, timbre, balance, edge);
    }
}

fn send_wave_channel_state(gb: &mut Gameboy, io_registers: &[u8]) {
    let nr30 = io_registers[0x1A];
    let nr51 = io_registers[0x25];

    let id = unsafe { (*gb.inner()).id };

    let mut volume: u8;
    let mut amplitude: u8;
    let period: u16;
    let mut edge: bool;
    let mut wave_table = [0u8; 32];
    unsafe {
        volume = GB_get_channel_volume(gb.as_mut_ptr(), ApuChannel::Wave.into());
        amplitude = GB_get_channel_amplitude(gb.as_mut_ptr(), ApuChannel::Wave.into());
        period = GB_get_channel_period(gb.as_mut_ptr(), ApuChannel::Wave.into());
        edge = GB_get_channel_edge_triggered(gb.as_mut_ptr(), ApuChannel::Wave.into());
        GB_get_apu_wave_table(gb.as_mut_ptr(), wave_table.as_mut_ptr());
    }

    if wave_table.iter().all(|&s| s == wave_table[0]) || (nr30 & 0x80) == 0 {
        volume = 0;
        edge = true;
    }

    let frequency = 65536.0 / (2048.0 - (period as f64));

    let mut hasher = DefaultHasher::new();
    hasher.write(&wave_table);
    hasher.write(&wave_table);
    let timbre = (hasher.finish() & 0xFF) as usize;

    let mix = nr51 & 0x44;
    let mix_l = (mix & 0xF0) != 0;
    let mix_r = (mix & 0x0F) != 0;

    let balance = match (mix_l, mix_r) {
        (false, false) => {
            volume = 0;
            amplitude = 0;
            0.5
        },
        (true, false) => 0.0,
        (false, true) => 1.0,
        (true, true) => 0.5
    };

    unsafe {
        (*gb.inner_mut()).apu_receiver
            .clone()
            .unwrap()
            .lock()
            .unwrap()
            .receive(id, ApuChannel::Wave, volume, amplitude, frequency, timbre, balance, edge);
    }
}

fn send_noise_channel_state(gb: &mut Gameboy, io_registers: &[u8]) {
    let nr43 = io_registers[0x22];
    let nr51 = io_registers[0x25];

    let id = unsafe { (*gb.inner()).id };

    let mut volume: u8;
    let mut amplitude: u8;
    let edge: bool;
    unsafe {
        volume = GB_get_channel_volume(gb.as_mut_ptr(), ApuChannel::Noise.into());
        amplitude = GB_get_channel_amplitude(gb.as_mut_ptr(), ApuChannel::Noise.into());
        edge = GB_get_channel_edge_triggered(gb.as_mut_ptr(), ApuChannel::Noise.into());
    }

    let clock_shift = (nr43 >> 4) as u16;
    let clock_divider = (nr43 & 7) as u16;
    let period = match clock_divider {
        0 => 0.5 * (1 << clock_shift) as f64,
        _ => (clock_divider << clock_shift) as f64
    };
    // Purely for visualizer aesthetic
    let lfsr_index = NOISE_PERIODS.iter().rev().position(|p| *p == period).unwrap();
    let frequency = C_0 * (2.0_f64).powf(lfsr_index as f64 / 69.0);

    // Timbre is just LFSR short mode
    let timbre = ((nr43 >> 3) & 1) as usize;

    let mix = nr51 & 0x88;
    let mix_l = (mix & 0xF0) != 0;
    let mix_r = (mix & 0x0F) != 0;

    let balance = match (mix_l, mix_r) {
        (false, false) => {
            volume = 0;
            amplitude = 0;
            0.5
        },
        (true, false) => 0.0,
        (false, true) => 1.0,
        (true, true) => 0.5
    };

    unsafe {
        (*gb.inner_mut()).apu_receiver
            .clone()
            .unwrap()
            .lock()
            .unwrap()
            .receive(id, ApuChannel::Noise, volume, amplitude, frequency, timbre, balance, edge);
    }
}

extern fn apu_sample_callback(gb: *mut GB_gameboy_t, sample: *mut GB_sample_t) {
    unsafe {
        let mut gb = Gameboy::wrap(gb);
        (*gb.inner_mut()).audio_buf.push_back((*sample).left);
        (*gb.inner_mut()).audio_buf.push_back((*sample).right);

        let io_registers = gb.get_io_registers();
        if (*gb.inner()).apu_receiver.is_some() {
            send_pulse_channel_state(&mut gb, false, &io_registers);
            send_pulse_channel_state(&mut gb, true, &io_registers);
            send_wave_channel_state(&mut gb, &io_registers);
            send_noise_channel_state(&mut gb, &io_registers);
        }
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub enum HighpassFilterMode {
    /// No filtering.
    Off,
    /// Applies a filter that is similar to the one used on hardware.
    #[default]
    Accurate,
    /// Only removes the DC offset without affecting the waveform.
    RemoveDCOffset
}

impl From<GB_highpass_mode_t> for HighpassFilterMode {
    fn from(value: GB_highpass_mode_t) -> Self {
        match value {
            GB_highpass_mode_t_GB_HIGHPASS_OFF => Self::Off,
            GB_highpass_mode_t_GB_HIGHPASS_ACCURATE => Self::Accurate,
            GB_highpass_mode_t_GB_HIGHPASS_REMOVE_DC_OFFSET => Self::RemoveDCOffset,
            _ => unreachable!("Invalid GB_highpass_mode_t value")
        }
    }
}

impl From<HighpassFilterMode> for GB_highpass_mode_t {
    fn from(value: HighpassFilterMode) -> Self {
        match value {
            HighpassFilterMode::Off => GB_highpass_mode_t_GB_HIGHPASS_OFF,
            HighpassFilterMode::Accurate => GB_highpass_mode_t_GB_HIGHPASS_ACCURATE,
            HighpassFilterMode::RemoveDCOffset => GB_highpass_mode_t_GB_HIGHPASS_REMOVE_DC_OFFSET
        }
    }
}

impl Gameboy {
    pub(crate) unsafe fn init_audio(gb: *mut GB_gameboy_t) {
        GB_apu_set_sample_callback(gb, Some(apu_sample_callback));
    }
}

impl Gameboy {
    /// Get the audio sample rate.
    pub fn get_sample_rate(&mut self) -> usize {
        unsafe {
            GB_get_sample_rate(self.as_mut_ptr()) as usize
        }
    }

    /// Set the audio sample rate.
    pub fn set_sample_rate(&mut self, sample_rate: usize) {
        unsafe {
            GB_set_sample_rate(self.as_mut_ptr(), sample_rate as _);
        }
    }

    /// Configure the high-pass filter.
    pub fn set_highpass_filter_mode(&mut self, highpass_mode: HighpassFilterMode) {
        unsafe {
            GB_set_highpass_filter_mode(self.as_mut_ptr(), highpass_mode.into());
        }
    }

    /// Set the audio interference volume.
    pub fn set_interference_volume(&mut self, interference_volume: f64) {
        unsafe {
            GB_set_interference_volume(self.as_mut_ptr(), interference_volume);
        }
    }

    /// Pop samples from the audio buffer.
    /// If you specify a frame size, then that many samples will be returned each invocation.
    /// If the buffer is underfull, then None is returned.
    /// If no frame size is specified, then the entire buffer is returned.
    pub fn get_audio_samples(&mut self, frame_size: Option<usize>) -> Option<Vec<i16>> {
        match frame_size {
            Some(frame_size) => unsafe {
                if (*self.inner()).audio_buf.len() < frame_size * 2 {
                    return None;
                }
                let result: Vec<_> = (*self.inner_mut()).audio_buf.drain(0..(frame_size * 2)).collect();
                Some(result)
            },
            None => unsafe {
                let result: Vec<_> = (*self.inner_mut()).audio_buf.clone().into_iter().collect();
                (*self.inner_mut()).audio_buf.clear();
                Some(result)
            }
        }
    }

    /// Check to see if an APU channel is muted.
    pub fn apu_channel_muted(&mut self, channel: ApuChannel) -> bool {
        unsafe {
            GB_is_channel_muted(self.as_mut_ptr(), channel.into())
        }
    }

    /// Mute/unmute an APU channel.
    pub fn apu_set_channel_muted(&mut self, channel: ApuChannel, muted: bool) {
        unsafe {
            GB_set_channel_muted(self.as_mut_ptr(), channel.into(), muted);
        }
    }

    /// Set an APU receiver to get updates on the currently playing audio.
    pub fn set_apu_receiver(&mut self, apu_receiver: Option<Arc<Mutex<dyn ApuStateReceiver>>>) {
        unsafe {
            (*self.inner_mut()).apu_receiver = apu_receiver;
        }
    }
}
