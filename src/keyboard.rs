use std::collections::HashSet;

use minifb::Key;
use tracing::{debug, info};
use u4::{U4x2, U4};

pub struct KeysChange {
    pub pressed: Vec<Key>,
    pub released: Vec<Key>,
}

type KeysPressedReceiver = std::sync::mpsc::Receiver<KeysChange>;

pub struct Keyboard {
    pressed_keys: HashSet<u4::U4>,
    key_receiver: KeysPressedReceiver,
}

impl Keyboard {
    pub fn new(key_receiver: KeysPressedReceiver) -> Self {
        return Self {
            pressed_keys: HashSet::new(),
            key_receiver,
        };
    }

    pub fn is_key_pressed_or_held(&mut self, chip_8_key: &U4) -> bool {
        self.update_pressed_keys();
        return self.pressed_keys.contains(chip_8_key);
    }

    pub fn get_pressed_key(&mut self) -> Option<U4> {
        self.update_pressed_keys();
        return self.pressed_keys.iter().next().cloned();
    }

    fn update_pressed_keys(&mut self) {
        while let Ok(changed_keys) = self.key_receiver.try_recv() {
            for pressed in changed_keys.pressed.iter() {
                if let Some(pressed_chip_8_key) = to_chip_8_key(*pressed) {
                    debug!("keyboard insert: {:?}", pressed_chip_8_key);
                    self.pressed_keys.insert(pressed_chip_8_key);
                }
            }
            for released in changed_keys.released.iter() {
                if let Some(released_chip_8_key) = to_chip_8_key(*released) {
                    debug!("keyboard remove: {:?}", released_chip_8_key);
                    self.pressed_keys.remove(&released_chip_8_key);
                }
            }
        }
    }
}

fn to_chip_8_key(key: Key) -> Option<U4> {
    if is_valid_key_code(key) {
        return Some(U4x2::from(key as u8).right());
    } else {
        info!("Unexpected input character {:#02x}", key as u8);
        return None;
    }
}

fn is_valid_key_code(key: Key) -> bool {
    return key as u8 <= Key::F as u8;
}
