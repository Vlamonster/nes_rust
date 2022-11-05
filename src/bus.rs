use crate::cartridge::Rom;
use crate::cpu::Mem;
use crate::ppu::PPU;

pub struct Bus {
    cpu_ram: [u8; 0x0800],
    prg_rom: Vec<u8>,
    ppu: PPU,
}

impl Bus {
    pub fn new(rom: Rom) -> Self {
        let ppu = PPU::new(rom.chr_rom, rom.screen_mirroring);

        Bus {
            cpu_ram: [0; 0x0800],
            prg_rom: rom.prg_rom,
            ppu,
        }
    }

    pub fn tick(&mut self, cycles: u8) {
        //self.cycles += cycles;
        self.ppu.tick(3 * cycles);
    }

    pub fn get_nmi(&mut self) -> bool {
        self.ppu.get_nmi()
    }
}

impl Mem for Bus {
    fn read(&mut self, adr: u16) -> u8 {
        match adr {
            0x0000..=0x1fff => self.cpu_ram[adr as usize & 0x07ff],
            0x2000..=0x3fff => match adr & 0x2007 {
                0x2002 => self.ppu.read_status(),
                0x2004 => self.ppu.read_oam_data(),
                0x2007 => self.ppu.read_data(),
                _ => panic!("Attempted to read from write-only PPU register {:x}", adr),
            },
            0x8000..=0xffff => {
                if self.prg_rom.len() == 0x4000 {
                    self.prg_rom[adr as usize & 0x3fff]
                } else {
                    self.prg_rom[adr as usize - 0x8000]
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
            0x2000..=0x3fff => match adr & 0x2007 {
                0x2000 => self.ppu.write_control(data),
                0x2001 => self.ppu.write_mask(data),
                0x2002 => panic!("Attempted to write to PPU status register"),
                0x2003 => self.ppu.write_oam_address(data),
                0x2004 => self.ppu.write_oam_data(data),
                0x2005 => todo!("write to scr not implemented yet"),
                0x2006 => self.ppu.write_address(data),
                0x2007 => self.ppu.write_data(data),
                _ => unreachable!(),
            },
            0x8000..=0xffff => {
                panic!("Attempted to write to Cartridge ROM space")
            }
            _ => {
                println!("Ignoring mem write-access at {:#x}", adr);
            }
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::cartridge::test::test_rom;

    #[test]
    fn test_read_write_ram() {
        let mut bus = Bus::new(test_rom(vec![0; 0x8000]));
        bus.write(0x01, 0x55);
        assert_eq!(bus.read(0x01), 0x55);
    }
}
