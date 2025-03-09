use anyhow::{anyhow, Result};
use minifb::{Key, KeyRepeat, Scale, ScaleMode, Window, WindowOptions};
use std::{
    cell::RefCell,
    env::{self},
    fs,
};

use cpu::Cpu;
use keyboard::Keyboard;
use renderer::{Renderer, SCREEN_HEIGHT, SCREEN_WIDTH};

mod cpu;
mod instruction;
mod keyboard;
mod memory;
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
    //let size = LogicalSize::new(SCREEN_WIDTH * 16, SCREEN_HEIGHT * 16);

    //let window_attributes = Window::default_attributes()
    //    .with_title("Chip-8 Emulator")
    //    .with_inner_size(size)
    //    .with_min_inner_size(size);
    //let window = event_loop
    //    .create_window(window_attributes)
    //    .expect("Should create window");
    //
    //let mut pixels = {
    //    //let window_size = window.inner_size();
    //    let surface_texture = SurfaceTexture::new(SCREEN_WIDTH, SCREEN_HEIGHT, &window);
    //    Pixels::new(SCREEN_WIDTH, SCREEN_HEIGHT, surface_texture).expect("Should create new pixels")
    //};

    let renderer = RefCell::new(Renderer::new());
    let keyboard = RefCell::new(Keyboard::new());
    let mut cpu = Cpu::new(&renderer, &keyboard);

    //let program = load_rom("./roms/1-chip8-logo.ch8").expect("rom should be loaded");
    //let expected_cycles = 39;
    //let program = load_rom("./roms/2-ibm-logo.ch8").expect("rom should be loaded");
    //let expected_cycles = 20;
    let expected_cycles = 10000;
    //let program = load_rom("./roms/BLINKY").expect("rom should be loaded");
    //let expected_cycles = 1000000000;
    cpu.load_program_into_memory(&rom);

    let mut cycle_count = 0;

    let mut frame_buffer: [u32; SCREEN_WIDTH * SCREEN_HEIGHT] = [0; SCREEN_WIDTH * SCREEN_HEIGHT];
    while window.is_open() && !window.is_key_down(Key::Escape) {
        if cycle_count < expected_cycles {
            cpu.run_cycle();
        }
        cycle_count += 1;

        keyboard
            .borrow_mut()
            .process_keyboard_event(window.get_keys_pressed(KeyRepeat::Yes));

        renderer.borrow_mut().update_pixels(&mut frame_buffer);
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
