use std::collections::HashSet;

use minifb::Key;

//const VALID_KEYS: [char; 16] = [
//    "1", "2", "3", "4", "5", "6", "7", "8", "9", "0", "a", "b", "c", "d", "e", "f",
//];

pub struct Keyboard {
    pressed_keys: HashSet<u8>,
}

impl Keyboard {
    pub fn new() -> Self {
        return Self {
            pressed_keys: HashSet::new(),
        };
    }

    pub fn is_key_pressed_or_held(&self, chip_8_key: &u8) -> bool {
        return self.pressed_keys.contains(chip_8_key);
    }

    pub fn get_pressed_key(&self) -> Option<u8> {
        return self.pressed_keys.iter().next().cloned();
    }

    pub(crate) fn process_keyboard_event(&mut self, pressed: Vec<Key>) {
        self.pressed_keys.clear();
        for key in pressed {
            if let Some(chip_8_key) = to_chip_8_key(key) {
                self.pressed_keys.insert(chip_8_key);
            }
        }
    }
}

fn to_chip_8_key(key: Key) -> Option<u8> {
    if key as u8 <= Key::F as u8 {
        return Some(key as u8);
    } else {
        println!("Unexpected input character {}", key as u8);
        return None;
    }
}
