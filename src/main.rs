use anyhow::{anyhow, Result};
use std::{cell::RefCell, fs, time::Instant};

use cpu::Cpu;
use keyboard::Keyboard;
use pixels::{Pixels, SurfaceTexture};
use renderer::{Renderer, SCREEN_HEIGHT, SCREEN_WIDTH};
use winit::{
    dpi::LogicalSize,
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::Window,
};

mod cpu;
mod keyboard;
mod memory;
mod renderer;

fn main() {
    let event_loop = EventLoop::new().expect("Should create EventLoop");
    // ControlFlow::Poll continuously runs the event loop, even if the OS hasn't
    // dispatched any events. This is ideal for games and similar applications.
    event_loop.set_control_flow(ControlFlow::Poll);
    let size = LogicalSize::new(SCREEN_WIDTH * 16, SCREEN_HEIGHT * 16);

    let window_attributes = Window::default_attributes()
        .with_title("Chip-8 Emulator")
        .with_inner_size(size)
        .with_min_inner_size(size);
    let window = event_loop
        .create_window(window_attributes)
        .expect("Should create window");

    let mut pixels = {
        let window_size = window.inner_size();
        let surface_texture = SurfaceTexture::new(window_size.width, window_size.height, &window);
        Pixels::new(SCREEN_WIDTH, SCREEN_HEIGHT, surface_texture).expect("Should create new pixels")
    };

    let renderer = RefCell::new(Renderer::new());
    let keyboard = RefCell::new(Keyboard::new());
    let mut cpu = Cpu::new(&renderer, &keyboard);

    //let program = load_rom("./roms/1-chip8-logo.ch8").expect("rom should be loaded");
    //let expected_cycles = 39;
    //let program = load_rom("./roms/2-ibm-logo.ch8").expect("rom should be loaded");
    //let expected_cycles = 20;
    let program = load_rom("./roms/3-corax+.ch8").expect("rom should be loaded");
    let expected_cycles = 300;
    cpu.load_program_into_memory(&program);

    let mut timer_started = Instant::now();
    let mut cycle_count = 0;

    let res = event_loop.run(|event, elwt| {
        if timer_started.elapsed().as_millis() >= (1000 / 60) {
            cpu.progress_timers();
            timer_started = Instant::now();
        }
        if cycle_count < expected_cycles {
            cpu.run_cycle();
        }
        cycle_count += 1;

        if let Event::WindowEvent { window_id, event } = event {
            match event {
                WindowEvent::RedrawRequested => {
                    renderer.borrow_mut().update_pixels(pixels.frame_mut());
                    if let Err(err) = pixels.render() {
                        println!("pixels.render error");
                        elwt.exit();
                        return;
                    }
                }
                WindowEvent::KeyboardInput {
                    device_id,
                    event,
                    is_synthetic,
                } => keyboard.borrow_mut().process_keyboard_event(event),
                _ => (),
            }
        }
        window.request_redraw();
    });
}

fn load_rom(file_path: &str) -> Result<Vec<u8>> {
    return fs::read(file_path).map_err(|e| anyhow!(e));
}
