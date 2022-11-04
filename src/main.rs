#![allow(dead_code)]

mod bus;
mod cartridge;
pub mod cpu;
pub mod opcodes;
mod trace;

use crate::bus::Bus;
use crate::cartridge::Rom;
use crate::cpu::{Mem, CPU};
use crate::trace::trace;
use rand::Rng;
use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::pixels::Color;
use sdl2::pixels::PixelFormatEnum;
use sdl2::EventPump;
use std::{fs, time};

fn color(byte: u8) -> Color {
    match byte {
        0 => Color::BLACK,
        1 => Color::WHITE,
        2 | 9 => Color::GREY,
        3 | 10 => Color::RED,
        4 | 11 => Color::GREEN,
        5 | 12 => Color::BLUE,
        6 | 13 => Color::MAGENTA,
        7 | 14 => Color::YELLOW,
        _ => Color::CYAN,
    }
}

fn read_screen_state(cpu: &CPU, frame: &mut [u8; 32 * 3 * 32]) -> bool {
    let mut frame_idx = 0;
    let mut update = false;
    for i in 0x0200..0x600 {
        let color_idx = cpu.read(i as u16);
        let (b1, b2, b3) = color(color_idx).rgb();
        if frame[frame_idx] != b1 || frame[frame_idx + 1] != b2 || frame[frame_idx + 2] != b3 {
            frame[frame_idx] = b1;
            frame[frame_idx + 1] = b2;
            frame[frame_idx + 2] = b3;
            update = true;
        }
        frame_idx += 3;
    }
    update
}

fn handle_user_input(cpu: &mut CPU, event_pump: &mut EventPump) {
    for event in event_pump.poll_iter() {
        match event {
            Event::Quit { .. }
            | Event::KeyDown {
                keycode: Some(Keycode::Escape),
                ..
            } => std::process::exit(0),
            Event::KeyDown {
                keycode: Some(Keycode::W),
                ..
            } => {
                cpu.write(0xff, 0x77);
            }
            Event::KeyDown {
                keycode: Some(Keycode::S),
                ..
            } => {
                cpu.write(0xff, 0x73);
            }
            Event::KeyDown {
                keycode: Some(Keycode::A),
                ..
            } => {
                cpu.write(0xff, 0x61);
            }
            Event::KeyDown {
                keycode: Some(Keycode::D),
                ..
            } => {
                cpu.write(0xff, 0x64);
            }
            _ => { /* do nothing */ }
        }
    }
}

fn main() {
    // init sdl2
    let sdl_context = sdl2::init().unwrap();
    let video_subsystem = sdl_context.video().unwrap();
    let window = video_subsystem
        .window("Snake game", (32.0 * 10.0) as u32, (32.0 * 10.0) as u32)
        .position_centered()
        .build()
        .unwrap();

    let mut canvas = window.into_canvas().present_vsync().build().unwrap();
    let mut event_pump = sdl_context.event_pump().unwrap();
    canvas.set_scale(10.0, 10.0).unwrap();
    let creator = canvas.texture_creator();
    let mut texture = creator
        .create_texture_target(PixelFormatEnum::RGB24, 32, 32)
        .unwrap();

    // load the game
    let bytes = fs::read("nestest.nes").unwrap();
    let rom = Rom::new(&bytes);
    let bus = Bus::new(rom);
    let mut cpu = CPU::new(bus);
    cpu.reset();
    cpu.pc = 0xc000;

    // run the game cycle
    // let mut screen_state = [0; 32 * 3 * 32];
    // let mut rng = rand::thread_rng();

    cpu.run_with_callback(
        move |cpu| {
            println!("{}", trace(cpu));
            // handle_user_input(cpu, &mut event_pump);
            // cpu.write(0xfe, rng.gen_range(1, 16));
            //
            // if read_screen_state(cpu, &mut screen_state) {
            //     texture.update(None, &screen_state, 32 * 3).unwrap();
            //     canvas.copy(&texture, None, None).unwrap();
            //     canvas.present();
            // }
            //
            // spin_sleep::sleep(time::Duration::from_nanos(70_000));
        },
        false,
        0,
    );
}
