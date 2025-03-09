pub const SCREEN_WIDTH: usize = 64;
pub const SCREEN_HEIGHT: usize = 32;

const SPRITE_WIDTH: usize = 8;

pub type DisplaySender = single_value_channel::Updater<Option<[[bool; 64]; 32]>>;

pub struct Renderer {
    display_content2d: [[bool; SCREEN_WIDTH]; SCREEN_HEIGHT],
    display_sender: DisplaySender,
}

impl Renderer {
    pub fn new(display_sender: DisplaySender) -> Self {
        return Renderer {
            display_content2d: [[false; 64]; 32],
            display_sender,
        };
    }

    pub fn clear_display(&mut self) {
        for line in self.display_content2d.iter_mut() {
            for pixel in line.iter_mut() {
                *pixel = false;
            }
        }
    }

    pub fn draw_sprite(&mut self, sprite: &[u8], target_x: u8, target_y: u8) -> bool {
        let mut pixel_erased = false;
        // wrapping around the display when the target location is out of bound
        let normalized_x = target_x as usize % SCREEN_WIDTH;
        let normalized_y = target_y as usize % SCREEN_HEIGHT;
        for (sprite_y, sprite_line_byte) in sprite.iter().enumerate() {
            for bit_index in (0..SPRITE_WIDTH).rev() {
                let pixel_x = normalized_x + SPRITE_WIDTH - 1 - bit_index;
                let pixel_y = normalized_y + sprite_y;
                if pixel_x >= SCREEN_WIDTH || pixel_y >= SCREEN_HEIGHT {
                    // the pixel would be out of screen there in wrapping around in this case
                    continue;
                }

                let bit_mask = 1 << bit_index;
                let masked = sprite_line_byte & bit_mask;
                let bit_set = masked != 0;
                let pixel = self.display_content2d[pixel_y][pixel_x];
                if pixel && pixel != bit_set {
                    pixel_erased = true
                }
                self.display_content2d[pixel_y][pixel_x] = bit_set;
            }
        }

        if !self.display_sender.has_no_receiver() {
            let update_result = self.display_sender.update(Some(self.display_content2d));
            if update_result.is_err() {
                println!("Failed to sent display update");
            }
        }

        return pixel_erased;
    }
}
