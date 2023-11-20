use sameboy_sys::{GB_has_accelerometer, GB_set_accelerometer_values};
use super::super::Gameboy;

impl Gameboy {
    /// Check if the current cartridge has an accelerometer.
    pub fn has_accelerometer(&mut self) -> bool {
        unsafe {
            GB_has_accelerometer(self.as_mut_ptr())
        }
    }

    /// Feed the emulated accelerometer with new data in units of
    /// gravitational acceleration (1 g ~= 9.8 m/s^2 = 9.8 N/kg).
    /// The magnitude of the values should not exceed 4 gs.
    pub fn set_accelerometer_values(&mut self, x: f64, y: f64) {
        unsafe {
            GB_set_accelerometer_values(self.as_mut_ptr(), x, y);
        }
    }
}
