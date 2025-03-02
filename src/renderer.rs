pub const SCREEN_WIDTH: u32 = 64;
pub const SCREEN_HEIGHT: u32 = 32;

const SPRITE_SIZE_BYTES: usize = 5;
const SPRITE_WIDTH: usize = 8;

pub struct Renderer {
    display_content2d: [[bool; SCREEN_WIDTH as usize]; SCREEN_HEIGHT as usize],
}

impl Renderer {
    pub fn new() -> Self {
        return Renderer {
            display_content2d: [[false; 64]; 32],
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
        for (sprite_y, sprite_line_byte) in sprite.iter().enumerate() {
            for bit_index in (0..SPRITE_WIDTH).rev() {
                let bit_mask = 1 << bit_index;
                let masked = sprite_line_byte & bit_mask;
                let bit_set = masked != 0;

                // wrapping around the display when the target location is out of bound
                let pixel_x =
                    (target_x as usize + (SPRITE_WIDTH - 1 - bit_index)) % SCREEN_WIDTH as usize;
                let pixel_y = (target_y as usize + sprite_y) % SCREEN_HEIGHT as usize;
                let pixel = self.display_content2d[pixel_y][pixel_x];
                if pixel && pixel != bit_set {
                    pixel_erased = true
                }
                self.display_content2d[pixel_y][pixel_x] = bit_set;
            }
        }
        return pixel_erased;
    }

    pub fn update_pixels(&self, frame: &mut [u8]) {
        for (i, frame_rgba) in frame.chunks_exact_mut(4).enumerate() {
            let x = (i % SCREEN_WIDTH as usize);
            let y = (i / SCREEN_WIDTH as usize);

            let rgba = if self.display_content2d[y][x] {
                [0x5e, 0x48, 0xe8, 0xff]
            } else {
                [0x48, 0xb2, 0xe8, 0xff]
            };

            frame_rgba.copy_from_slice(&rgba);
        }
    }
}
