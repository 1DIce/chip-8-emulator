use anyhow::{anyhow, Result};
use minifb::{Key, KeyRepeat, Scale, ScaleMode, Window, WindowOptions};
use std::{
    cell::RefCell,
    env::{self},
    fs,
    sync::{mpsc, Arc, Mutex},
    thread,
};

use cpu::Cpu;
use keyboard::Keyboard;
use renderer::{Renderer, SCREEN_HEIGHT, SCREEN_WIDTH};

mod audio;
mod cpu;
mod instruction;
mod keyboard;
mod memory;
mod program_counter;
mod renderer;

fn main() -> Result<()> {
    let args: Vec<String> = env::args().collect();

    let rom: Vec<u8> = if args.len() > 1 {
        load_rom(&args[1])?
    } else {
        load_rom("./roms/1-chip8-logo.ch8")?
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
    let (keyboard_receiver, pressed_keys_sender) = single_value_channel::channel();

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
        if !pressed_keys_sender.has_no_receiver() {
            pressed_keys_sender.update(Some(window.get_keys_pressed(KeyRepeat::Yes)))?;
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
            0x5e << 16 | 0x48 << 8 | 0xe8
        } else {
            0x48 << 16 | 0xb2 << 8 | 0xe8
        };

        *frame_rgb = rgb;
    }
}
