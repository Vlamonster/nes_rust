use crate::cpu::Mem;

pub struct Bus {
    cpu_ram: [u8; 2048],
}

impl Bus {
    pub fn new() -> Self {
        Bus {
            cpu_ram: [0; 0x0800]
        }
    }
}

impl Mem for Bus {
    fn read(&self, adr: u16) -> u8 {
        match adr {
            0x0000..=0x1fff => {
                self.cpu_ram[adr as usize & 0x07ff]
            }
            0x2000..=0x3fff => {
                let _mirror_down_addr = adr & 0x2007;
                todo!("PPU is not supported yet")
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
            _ => {
                println!("Ignoring mem write-access at {}", adr);
            }
        }
    }
}