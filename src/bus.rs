use crate::cartridge::Rom;
use crate::cpu::Mem;

pub struct Bus {
    cpu_ram: [u8; 2048],
    rom: Rom,
}

impl Bus {
    pub fn new(rom: Rom) -> Self {
        Bus {
            cpu_ram: [0; 0x0800],
            rom,
        }
    }
}

impl Mem for Bus {
    fn read(&self, adr: u16) -> u8 {
        match adr {
            0x0000..=0x1fff => self.cpu_ram[adr as usize & 0x07ff],
            0x2000..=0x3fff => {
                let _mirror_down_addr = adr & 0x2007;
                todo!("PPU is not supported yet")
            }
            0x8000..=0xffff => {
                if self.rom.prg_rom.len() == 0x4000 {
                    self.rom.prg_rom[adr as usize & 0x3fff]
                } else {
                    self.rom.prg_rom[adr as usize - 0x8000]
                }
            }
            _ => {
                println!("Ignoring mem access at {}", adr);
                0
            }
        }
    }

    fn write(&mut self, adr: u16, data: u8) {
        match adr {
            0x0000..=0x1fff => {
                self.cpu_ram[adr as usize & 0x07ff] = data;
            }
            0x2000..=0x3fff => {
                let _mirror_down_adr = adr & 0x2007;
                todo!("PPU is not supported yet");
            }
            0x8000..=0xffff => {
                panic!("Attempt to write to Cartridge ROM space")
            }
            _ => {
                println!("Ignoring mem write-access at {}", adr);
            }
        }
    }
}
