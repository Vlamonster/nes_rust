use crate::cartridge::Rom;
use crate::cpu::Mem;
use crate::joypad::Joypad;
use crate::ppu::PPU;

pub struct Bus<'call> {
    cpu_ram: [u8; 0x0800],
    prg_rom: Vec<u8>,
    pub ppu: PPU,
    joypad_1: Joypad,

    callback: Box<dyn FnMut(&PPU, &mut Joypad) + 'call>,
}

impl<'a> Bus<'a> {
    pub fn new<'call, F>(rom: Rom, callback: F) -> Bus<'call>
    where
        F: FnMut(&PPU, &mut Joypad) + 'call,
    {
        let ppu = PPU::new(rom.chr_rom, rom.screen_mirroring);

        Bus {
            cpu_ram: [0; 0x0800],
            prg_rom: rom.prg_rom,
            ppu,
            joypad_1: Joypad::new(),

            callback: Box::from(callback),
        }
    }

    pub fn tick(&mut self, cycles: u8) {
        //self.cycles += cycles;
        if self.ppu.tick(3 * cycles) {
            (self.callback)(&self.ppu, &mut self.joypad_1);
        }
    }

    pub fn get_nmi(&mut self) -> bool {
        self.ppu.get_nmi()
    }
}

impl Mem for Bus<'_> {
    fn read(&mut self, adr: u16) -> u8 {
        match adr {
            0x0000..=0x1fff => self.cpu_ram[adr as usize & 0x07ff],
            0x2000..=0x3fff => match adr & 0x2007 {
                0x2002 => self.ppu.read_status(),
                0x2004 => self.ppu.read_oam_data(),
                0x2007 => self.ppu.read_data(),
                _ => panic!("Attempted to read from write-only PPU register {:x}", adr),
            },
            0x4000..=0x4015 => {
                // todo implement APU, return 0 for now
                0
            }
            0x4016 => self.joypad_1.read(),
            0x4017 => {
                // todo implement joy pad 2 read, return 0 for now
                0
            }
            0x8000..=0xffff => {
                if self.prg_rom.len() == 0x4000 {
                    self.prg_rom[adr as usize & 0x3fff]
                } else {
                    self.prg_rom[adr as usize - 0x8000]
                }
            }
            _ => {
                println!("Ignoring mem access at {:#x}", adr);
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
                0x2005 => self.ppu.write_scroll(data),
                0x2006 => self.ppu.write_address(data),
                0x2007 => self.ppu.write_data(data),
                _ => unreachable!(),
            },
            0x4000..=0x4013 | 0x4015 => {
                // ignore APU
            }
            0x4014 => {
                let mut buffer: [u8; 256] = [0; 256];
                let hi: u16 = (data as u16) << 8;
                for i in 0x00..=0xffu16 {
                    buffer[i as usize] = self.read(hi + i);
                }

                self.ppu.write_oam_dma(&buffer);
            }
            0x4016 => self.joypad_1.write(data),
            0x4017 => {
                // ignore joy pad 2
            }
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
        let mut bus = Bus::new(test_rom(vec![0; 0x8000]), |_, _| {});
        bus.write(0x01, 0x55);
        assert_eq!(bus.read(0x01), 0x55);
    }
}
