use std::cell::RefCell;
use std::collections::hash_map::DefaultHasher;
use std::hash::Hasher;
use std::rc::Rc;
use sameboy_sys::{GB_apu_set_sample_callback, GB_channel_t, GB_channel_t_GB_NOISE, GB_channel_t_GB_SQUARE_1, GB_channel_t_GB_SQUARE_2, GB_channel_t_GB_WAVE, GB_gameboy_t, GB_get_apu_wave_table, GB_get_channel_amplitude, GB_get_channel_edge_triggered, GB_get_channel_period, GB_get_channel_volume, GB_is_channel_muted, GB_sample_t, GB_set_channel_muted, GB_set_sample_rate};
use super::Gameboy;

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
    /// - Amplitude (0-15)
    /// - Frequency (Hz)
    /// - Timbre (arbitrary index, e.g. for selecting a color)
    /// - Balance (0.0-1.0, 0.0=left, 0.5=center, 1.0=right)
    fn receive(&mut self, channel: ApuChannel, volume: u8, amplitude: u8, frequency: f64, timbre: usize, balance: f64, edge: bool);
}

fn send_pulse_channel_state(gb: &mut Gameboy, pulse2: bool, io_registers: &[u8]) {
    let io_base = if pulse2 { 0x15 } else { 0x10 };
    let nrx1 = io_registers[io_base + 1];
    // let nrx3 = io_registers[io_base + 3];
    // let nrx4 = io_registers[io_base + 4];
    // let nr50 = io_registers[0x24];
    let nr51 = io_registers[0x25];
    // let pcm12 = io_registers[0x76];  // FIXME: CGB only

    let channel = if pulse2 { ApuChannel::Pulse2 } else { ApuChannel::Pulse1 };

    // let mut amplitude = pcm12;
    // if pulse2 {
    //     amplitude >>= 4;
    // }
    // amplitude &= 0xF;

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
    // TODO: skew balance with nr50

    gb.apu_receiver.clone().unwrap().borrow_mut().receive(channel, volume, amplitude, frequency, timbre, balance, edge);
}

fn send_wave_channel_state(gb: &mut Gameboy, io_registers: &[u8]) {
    let nr30 = io_registers[0x1A];
    // let nr32 = io_registers[0x1C];
    // let nr33 = io_registers[0x1D];
    // let nr34 = io_registers[0x1E];
    // let nr50 = io_registers[0x24];
    let nr51 = io_registers[0x25];
    // let pcm34 = io_registers[0x77];  // FIXME: CGB only

    // normalize to the scale of the other channels
    // let volume = match nr32 & 0x60 {
    //     0b0_00_00000 => 0x0,
    //     0b0_01_00000 => 0xF,
    //     0b0_10_00000 => 0x8,
    //     0b0_11_00000 => 0x4,
    //     _ => unreachable!()
    // };

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

    // let period = (((nr34 & 7) as u16) << 8) | (nr33 as u16);
    // TODO: account for wave pattern
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
    // TODO: skew balance with nr50

    gb.apu_receiver.clone().unwrap().borrow_mut().receive(ApuChannel::Wave, volume, amplitude, frequency, timbre, balance, edge);
}

fn send_noise_channel_state(gb: &mut Gameboy, io_registers: &[u8]) {
    let nr43 = io_registers[0x22];
    // let nr50 = io_registers[0x24];
    let nr51 = io_registers[0x25];
    // let pcm34 = io_registers[0x77];  // FIXME: CGB only

    // let mut amplitude = pcm34 >> 4;

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
    // let frequency = (262144.0 / period).sqrt() / 4.0;
    // let frequency = 17.351597831287 + (period.log2() / 2.0);
    let lfsr_index = NOISE_PERIODS.iter().position(|p| *p == period).unwrap();
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
    // TODO: skew balance with nr50

    gb.apu_receiver.clone().unwrap().borrow_mut().receive(ApuChannel::Noise, volume, amplitude, frequency, timbre, balance, edge);
}

extern fn apu_sample_callback(gb: *mut GB_gameboy_t, sample: *mut GB_sample_t) {
    unsafe {
        let gb = Gameboy::mut_from_callback_ptr(gb);
        gb.audio_buf.push_back((*sample).left);
        gb.audio_buf.push_back((*sample).right);

        let io_registers = gb.get_io_registers();
        if gb.apu_receiver.is_some() {
            send_pulse_channel_state(gb, false, &io_registers);
            send_pulse_channel_state(gb, true, &io_registers);
            send_wave_channel_state(gb, &io_registers);
            send_noise_channel_state(gb, &io_registers);
        }
    }
}

impl Gameboy {
    pub fn init_audio(&mut self) {
        unsafe {
            GB_apu_set_sample_callback(self.as_mut_ptr(), Some(apu_sample_callback))
        }
    }

    /// Set the audio sample rate.
    pub fn set_sample_rate(&mut self, sample_rate: usize) {
        unsafe {
            GB_set_sample_rate(self.as_mut_ptr(), sample_rate as _);
        }
    }

    /// Pop samples from the audio buffer.
    /// If you specify a frame size, then that many samples will be returned each invocation.
    /// If the buffer is underfull, then None is returned.
    /// If no frame size is specified, then the entire buffer is returned.
    pub fn get_audio_samples(&mut self, frame_size: Option<usize>) -> Option<Vec<i16>> {
        match frame_size {
            Some(frame_size) => {
                if self.audio_buf.len() < frame_size * 2 {
                    return None;
                }
                let result: Vec<_> = self.audio_buf.drain(0..(frame_size * 2)).collect();
                Some(result)
            },
            None => {
                let result: Vec<_> = self.audio_buf.clone().into_iter().collect();
                self.audio_buf.clear();
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

    pub fn set_apu_receiver(&mut self, apu_receiver: Option<Rc<RefCell<dyn ApuStateReceiver>>>) {
        self.apu_receiver = apu_receiver;
    }
}
