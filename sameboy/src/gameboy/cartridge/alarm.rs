use std::time::Duration;
use sameboy_sys::GB_time_to_alarm;
use super::super::Gameboy;

impl Gameboy {
    /// If the cartridge has an alarm clock (e.g. HuC-3), get the time remaining
    /// until it is triggered. Returns 0 if no alarm is active.
    pub fn alarm_time_remaining(&mut self) -> Duration {
        unsafe {
            Duration::from_secs(GB_time_to_alarm(self.as_mut_ptr()) as u64)
        }
    }
}