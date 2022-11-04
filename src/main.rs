#![allow(dead_code)]

mod bus;
mod cartridge;
pub mod cpu;
pub mod opcodes;
mod ppu;
mod trace;

use crate::bus::Bus;
use crate::cartridge::Rom;
use crate::cpu::CPU;
use crate::trace::trace;
use std::fs;

fn main() {
    // load the game
    let bytes = fs::read("nestest.nes").unwrap();
    let rom = Rom::new(&bytes);
    let bus = Bus::new(rom);
    let mut cpu = CPU::new(bus);
    cpu.reset();
    cpu.pc = 0xc000;

    // run the game cycle
    cpu.run_with_callback(
        move |cpu| {
            println!("{}", trace(cpu));
        },
        false,
        0,
    );
}
