use anyhow::{anyhow, Result};
use logging::setup_logging;
use minifb::{Key, KeyRepeat, Scale, ScaleMode, Window, WindowOptions};
use std::{
    env::{self},
    fs, thread,
};
use tracing::{debug, info};

use cpu::Cpu;
use keyboard::Keyboard;
use renderer::{Renderer, SCREEN_HEIGHT, SCREEN_WIDTH};

mod audio;
mod cpu;
mod instruction;
mod keyboard;
mod logging;
mod memory;
mod program_counter;
mod renderer;

#[allow(clippy::eq_op, clippy::identity_op)]
const BACKGROUND_COLOR_RGB: u32 = 0x00 << 16 | 0x00 << 8 | 0x00;
#[allow(clippy::eq_op, clippy::identity_op)]
const FOREGROUND_COLOR_RGB: u32 = 0x00 << 16 | 0x99 << 8 | 0x00;

fn main() -> Result<()> {
    setup_logging();

    let args: Vec<String> = env::args().collect();

    let rom: Vec<u8> = if args.len() > 1 {
        load_rom(&args[1])?
    } else {
        info!("No rom provided, using default rom");
        load_rom("./roms/test/1-chip8-logo.ch8")?
    };

    let mut window = Window::new(
        "Chip-8 Emulator",
        SCREEN_WIDTH,
        SCREEN_HEIGHT,
        WindowOptions {
            resize: true,
            scale: Scale::X16,
            scale_mode: ScaleMode::AspectRatioStretch,
            ..WindowOptions::default()
        },
    )?;

    let (mut display_receiver, display_sender) = single_value_channel::channel();
    let (pressed_keys_sender, keyboard_receiver) = std::sync::mpsc::channel();

    let renderer = Renderer::new(display_sender);
    let keyboard = Keyboard::new(keyboard_receiver);

    let mut frame_buffer: [u32; SCREEN_WIDTH * SCREEN_HEIGHT] = [0; SCREEN_WIDTH * SCREEN_HEIGHT];

    thread::spawn(move || {
        let mut cpu = Cpu::new(renderer, keyboard);
        cpu.load_program_into_memory(&rom);
        loop {
            cpu.run_cycle();
        }
    });

    while window.is_open() && !window.is_key_down(Key::Escape) {
        let change = keyboard::KeysChange {
            pressed: window.get_keys_pressed(KeyRepeat::No),
            released: window.get_keys_released(),
        };
        if !change.released.is_empty() || !change.pressed.is_empty() {
            debug!("pressed: {:?}", change.pressed);
            debug!("released: {:?}", change.released);
            pressed_keys_sender.send(change)?;
        }

        if let Some(latest) = display_receiver.latest() {
            update_pixels(&mut frame_buffer, latest)
        }

        window.update_with_buffer(&frame_buffer, SCREEN_WIDTH, SCREEN_HEIGHT)?;
    }

    return Ok(());
}

fn load_rom(file_path: &str) -> Result<Vec<u8>> {
    if fs::exists(file_path).unwrap_or(false) {
        return fs::read(file_path).map_err(|e| anyhow!(e));
    }
    return Err(anyhow!("Rom file '{}' does not exist", file_path));
}

fn update_pixels(frame_buffer: &mut [u32], display_content: &[[bool; 64]; 32]) {
    for (i, frame_rgb) in frame_buffer.iter_mut().enumerate() {
        let x = i % SCREEN_WIDTH;
        let y = i / SCREEN_WIDTH;

        let rgb: u32 = if display_content[y][x] {
            FOREGROUND_COLOR_RGB
        } else {
            BACKGROUND_COLOR_RGB
        };

        *frame_rgb = rgb;
    }
}
