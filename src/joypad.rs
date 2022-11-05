pub const JOYPAD_A: u8 = 0b0000_0001;
pub const JOYPAD_B: u8 = 0b0000_0010;
pub const JOYPAD_SELECT: u8 = 0b0000_0100;
pub const JOYPAD_START: u8 = 0b0000_1000;
pub const JOYPAD_UP: u8 = 0b0001_0000;
pub const JOYPAD_DOWN: u8 = 0b0010_0000;
pub const JOYPAD_LEFT: u8 = 0b0100_0000;
pub const JOYPAD_RIGHT: u8 = 0b1000_0000;

pub struct Joypad {
    strobe: bool,
    button_index: u8,
    button_flags: u8,
}

impl Joypad {
    pub fn new() -> Self {
        Joypad {
            strobe: false,
            button_index: 0,
            button_flags: 0b0000_0000,
        }
    }

    pub fn write(&mut self, data: u8) {
        self.strobe = data & 0b0000_0001 != 0;
        if self.strobe {
            self.button_index = 0
        }
    }

    pub fn read(&mut self) -> u8 {
        if self.button_index > 7 {
            return 1;
        }
        let response = (self.button_flags & (1 << self.button_index)) >> self.button_index;
        if !self.strobe && self.button_index <= 7 {
            self.button_index += 1;
        }
        response
    }

    pub fn set_button_pressed_status(&mut self, button: u8, pressed: bool) {
        if pressed {
            self.button_flags |= button;
        } else {
            self.button_flags &= !button;
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_strobe_mode() {
        let mut joypad = Joypad::new();
        joypad.write(1);
        joypad.set_button_pressed_status(JOYPAD_A, true);
        for _x in 0..10 {
            assert_eq!(joypad.read(), 1);
        }
    }

    #[test]
    fn test_strobe_mode_on_off() {
        let mut joypad = Joypad::new();

        joypad.write(0);
        joypad.set_button_pressed_status(JOYPAD_RIGHT, true);
        joypad.set_button_pressed_status(JOYPAD_LEFT, true);
        joypad.set_button_pressed_status(JOYPAD_SELECT, true);
        joypad.set_button_pressed_status(JOYPAD_B, true);

        for _ in 0..=1 {
            assert_eq!(joypad.read(), 0);
            assert_eq!(joypad.read(), 1);
            assert_eq!(joypad.read(), 1);
            assert_eq!(joypad.read(), 0);
            assert_eq!(joypad.read(), 0);
            assert_eq!(joypad.read(), 0);
            assert_eq!(joypad.read(), 1);
            assert_eq!(joypad.read(), 1);

            for _x in 0..10 {
                assert_eq!(joypad.read(), 1);
            }
            joypad.write(1);
            joypad.write(0);
        }
    }
}
