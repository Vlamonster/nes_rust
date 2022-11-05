#![allow(dead_code)]

mod bus;
mod cartridge;
pub mod cpu;
pub mod opcodes;
mod ppu;
mod render;
mod trace;

use crate::bus::Bus;
use crate::cartridge::Rom;
use crate::cpu::CPU;
use crate::ppu::PPU;
use crate::render::{Frame, PALETTE};
use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::pixels::PixelFormatEnum;
use std::fs;

fn show_tile_bank(chr_rom: &[u8], bank: u8) -> Frame {
    if bank > 1 {
        panic!("There is no bank {}", bank);
    }

    let mut frame = Frame::new();

    let mut offset_x = 0;
    let mut offset_y = 0;
    let offset_rom = bank as usize * 0x1000;

    for tile_index in 0x00..=0xff {
        // Increment row every 32 tiles
        if tile_index != 0 && tile_index % 32 == 0 {
            offset_y += 8;
        }

        // Fetch tile bytes
        let tile = &chr_rom[(offset_rom + tile_index * 16)..=(offset_rom + tile_index * 16 + 15)];

        for y in 0..=7 {
            let color_hi = tile[y].reverse_bits();
            let color_lo = tile[y + 8].reverse_bits();

            for x in 0..=7 {
                let rgb = match ((color_hi >> x) & 1) << 1 | ((color_lo >> x) & 1) {
                    0 => PALETTE[0x01],
                    1 => PALETTE[0x23],
                    2 => PALETTE[0x27],
                    3 => PALETTE[0x30],
                    _ => unreachable!(),
                };
                frame.set_pixel(offset_x + x, offset_y + y, rgb)
            }
        }

        // Increment column every tile
        offset_x += 8;
    }
    frame
}

fn main() {
    // init sdl2
    let sdl_context = sdl2::init().unwrap();
    let video_subsystem = sdl_context.video().unwrap();
    let window = video_subsystem
        .window("Tile viewer", (256.0 * 3.0) as u32, (240.0 * 3.0) as u32)
        .position_centered()
        .build()
        .unwrap();

    let mut canvas = window.into_canvas().present_vsync().build().unwrap();
    let mut event_pump = sdl_context.event_pump().unwrap();
    canvas.set_scale(3.0, 3.0).unwrap();

    let creator = canvas.texture_creator();
    let mut texture = creator
        .create_texture_target(PixelFormatEnum::RGB24, 256, 240)
        .unwrap();

    //load the game
    let bytes: Vec<u8> = fs::read("pacman.nes").unwrap();
    let rom = Rom::new(&bytes);

    let mut frame = Frame::new();

    // the game cycle
    let bus = Bus::new(rom, move |ppu: &PPU| {
        render::render(ppu, &mut frame);
        texture.update(None, &frame.data, 256 * 3).unwrap();

        canvas.copy(&texture, None, None).unwrap();

        canvas.present();
        for event in event_pump.poll_iter() {
            match event {
                Event::Quit { .. }
                | Event::KeyDown {
                    keycode: Some(Keycode::Escape),
                    ..
                } => std::process::exit(0),
                _ => { /* do nothing */ }
            }
        }
    });

    let mut cpu = CPU::new(bus);

    cpu.reset();
    cpu.run(false, 0);
}
