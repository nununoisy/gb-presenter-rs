use sameboy::MemoryInterceptor;

pub struct EndDetector {
    sound_enabled: bool,
    sound_disabled: bool
}

impl EndDetector {
    pub fn new() -> Self {
        Self {
            sound_enabled: false,
            sound_disabled: false
        }
    }

    pub fn reset(&mut self) {
        self.sound_enabled = false;
        self.sound_disabled = false;
    }

    pub fn detected(&self) -> bool {
        self.sound_disabled
    }
}

impl MemoryInterceptor for EndDetector {
    fn intercept_write(&mut self, _id: usize, addr: u16, data: u8) -> bool {
        if addr == 0xFF26 {
            if !self.sound_enabled && data != 0 {
                self.sound_enabled = true;
            } else if self.sound_enabled && data == 0 {
                self.sound_disabled = true;
            }
        }
        true
    }
}
