use std::collections::HashSet;

use winit::{event::KeyEvent, keyboard::Key};

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

    pub(crate) fn process_keyboard_event(&mut self, event: KeyEvent) {
        if let Key::Character(key) = event.logical_key {
            if let Some(chip_8_key) = to_chip_8_key(key.as_str()) {
                if event.state.is_pressed() {
                    self.pressed_keys.insert(chip_8_key);
                } else {
                    self.pressed_keys.remove(&chip_8_key);
                }
            }
        }
    }
}

fn to_chip_8_key(key: &str) -> Option<u8> {
    if key.len() != 1 {
        return None;
    }

    let character = key.chars().next().expect("string is empty") as u8;
    if (48..=57).contains(&character) {
        return Some(character - 48);
    } else if (65..=70).contains(&character) {
        return Some(character - 55);
    } else {
        println!("Unexpected input character {}", key);
        return None;
    }
}
