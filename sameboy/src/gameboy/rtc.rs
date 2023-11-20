use sameboy_sys::{GB_rtc_mode_t, GB_rtc_mode_t_GB_RTC_MODE_ACCURATE, GB_rtc_mode_t_GB_RTC_MODE_SYNC_TO_HOST, GB_set_rtc_mode, GB_set_rtc_multiplier};
use super::Gameboy;

#[derive(Debug, Copy, Clone)]
pub enum RtcMode {
    /// RTC is synced to the host clock.
    SyncToHost,
    /// RTC is emulated accurately.
    Accurate
}

impl From<GB_rtc_mode_t> for RtcMode {
    fn from(value: GB_rtc_mode_t) -> Self {
        match value {
            GB_rtc_mode_t_GB_RTC_MODE_SYNC_TO_HOST => Self::SyncToHost,
            GB_rtc_mode_t_GB_RTC_MODE_ACCURATE => Self::Accurate,
            _ => unreachable!("Invalid GB_rtc_mode_t value")
        }
    }
}

impl From<RtcMode> for GB_rtc_mode_t {
    fn from(value: RtcMode) -> Self {
        match value {
            RtcMode::SyncToHost => GB_rtc_mode_t_GB_RTC_MODE_SYNC_TO_HOST,
            RtcMode::Accurate => GB_rtc_mode_t_GB_RTC_MODE_ACCURATE
        }
    }
}

impl Gameboy {
    /// Configure RTC emulation.
    pub fn set_rtc_mode(&mut self, rtc_mode: RtcMode) {
        unsafe {
            GB_set_rtc_mode(self.as_mut_ptr(), rtc_mode.into());
        }
    }

    /// Set the rate at which the RTC ticks forward.
    /// Useful mainly for TAS syncing.
    pub fn set_rtc_multiplier(&mut self, rtc_multiplier: f64) {
        unsafe {
            GB_set_rtc_multiplier(self.as_mut_ptr(), rtc_multiplier);
        }
    }
}
