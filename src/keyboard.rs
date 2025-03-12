use std::collections::HashSet;

use minifb::Key;

pub struct KeysChange {
    pub pressed: Vec<Key>,
    pub released: Vec<Key>,
}

type KeysPressedReceiver = single_value_channel::Receiver<Option<KeysChange>>;

pub struct Keyboard {
    pressed_keys: HashSet<u8>,
    key_receiver: KeysPressedReceiver,
}

impl Keyboard {
    pub fn new(key_receiver: KeysPressedReceiver) -> Self {
        return Self {
            pressed_keys: HashSet::new(),
            key_receiver,
        };
    }

    pub fn is_key_pressed_or_held(&mut self, chip_8_key: &u8) -> bool {
        self.update_pressed_keys();
        return self.pressed_keys.contains(chip_8_key);
    }

    pub fn get_pressed_key(&mut self) -> Option<u8> {
        self.update_pressed_keys();
        return self.pressed_keys.iter().next().cloned();
    }

    fn update_pressed_keys(&mut self) {
        if let Some(changed_keys) = self.key_receiver.latest() {
            for pressed in changed_keys.pressed.iter() {
                if let Some(pressed_chip_8_key) = to_chip_8_key(*pressed) {
                    self.pressed_keys.insert(pressed_chip_8_key);
                }
            }
            for released in changed_keys.released.iter() {
                if let Some(released_chip_8_key) = to_chip_8_key(*released) {
                    self.pressed_keys.remove(&released_chip_8_key);
                }
            }
        }
    }

    //pub(crate) fn process_keyboard_event(&mut self, pressed: Vec<Key>) {
    //    self.pressed_keys.clear();
    //    for key in pressed {
    //        if let Some(chip_8_key) = to_chip_8_key(key) {
    //            self.pressed_keys.insert(chip_8_key);
    //        }
    //    }
    //}
}

fn to_chip_8_key(key: Key) -> Option<u8> {
    if is_valid_key_code(key) {
        return Some(key as u8);
    } else {
        println!("Unexpected input character {}", key as u8);
        return None;
    }
}

fn is_valid_key_code(key: Key) -> bool {
    return key as u8 <= Key::F as u8;
}
