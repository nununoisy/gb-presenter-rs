use sameboy_sys::{GB_key_t, GB_key_t_GB_KEY_RIGHT, GB_key_t_GB_KEY_LEFT, GB_key_t_GB_KEY_UP, GB_key_t_GB_KEY_DOWN, GB_key_t_GB_KEY_A, GB_key_t_GB_KEY_B, GB_key_t_GB_KEY_SELECT, GB_key_t_GB_KEY_START, GB_set_key_state};
use super::Gameboy;

#[derive(Copy, Clone)]
pub enum JoypadButton {
    Right,
    Left,
    Up,
    Down,
    A,
    B,
    Select,
    Start
}

impl From<GB_key_t> for JoypadButton {
    fn from(value: GB_key_t) -> Self {
        match value {
            GB_key_t_GB_KEY_RIGHT => Self::Right,
            GB_key_t_GB_KEY_LEFT => Self::Left,
            GB_key_t_GB_KEY_UP => Self::Up,
            GB_key_t_GB_KEY_DOWN => Self::Down,
            GB_key_t_GB_KEY_A => Self::A,
            GB_key_t_GB_KEY_B => Self::B,
            GB_key_t_GB_KEY_SELECT => Self::Select,
            GB_key_t_GB_KEY_START => Self::Start,
            _ => unreachable!("Invalid GB_key_t value")
        }
    }
}

impl From<JoypadButton> for GB_key_t {
    fn from(value: JoypadButton) -> Self {
        match value {
            JoypadButton::Right => GB_key_t_GB_KEY_RIGHT,
            JoypadButton::Left => GB_key_t_GB_KEY_LEFT,
            JoypadButton::Up => GB_key_t_GB_KEY_UP,
            JoypadButton::Down => GB_key_t_GB_KEY_DOWN,
            JoypadButton::A => GB_key_t_GB_KEY_A,
            JoypadButton::B => GB_key_t_GB_KEY_B,
            JoypadButton::Select => GB_key_t_GB_KEY_SELECT,
            JoypadButton::Start => GB_key_t_GB_KEY_START
        }
    }
}

impl Gameboy {
    /// Press or release a button on the joypad.
    pub fn set_joypad_button(&mut self, button: JoypadButton, pressed: bool) {
        unsafe {
            GB_set_key_state(self.as_mut_ptr(), button.into(), pressed);
        }
    }

    /// Release all buttons on the joypad.
    pub fn joypad_release_all(&mut self) {
        self.set_joypad_button(JoypadButton::Right, false);
        self.set_joypad_button(JoypadButton::Left, false);
        self.set_joypad_button(JoypadButton::Up, false);
        self.set_joypad_button(JoypadButton::Down, false);
        self.set_joypad_button(JoypadButton::A, false);
        self.set_joypad_button(JoypadButton::B, false);
        self.set_joypad_button(JoypadButton::Select, false);
        self.set_joypad_button(JoypadButton::Start, false);
    }

    /// Press only the buttons in the list, and run the GameBoy for a frame.
    /// Useful for automation.
    pub fn joypad_macro_frame(&mut self, buttons: &[JoypadButton]) {
        self.joypad_release_all();
        for button in buttons {
            self.set_joypad_button(button.clone(), true);
        }
        self.run_frame();
    }
}
