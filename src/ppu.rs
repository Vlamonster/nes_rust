use crate::cartridge::Mirroring;
use crate::cartridge::Mirroring::{Horizontal, Vertical};

#[allow(clippy::upper_case_acronyms)]
pub struct PPU {
    pub chr_rom: Vec<u8>,
    pub palette_table: [u8; 32],
    pub vram: [u8; 2048],
    pub oam_data: [u8; 256],

    pub mirroring: Mirroring,

    pub buffer: u8,

    pub adr: PpuAdr,
    pub ctr: PpuCtr,
}

impl PPU {
    pub fn new(chr_rom: Vec<u8>, mirroring: Mirroring) -> Self {
        PPU {
            chr_rom,
            vram: [0; 0x0800],
            oam_data: [0; 256],
            palette_table: [0; 32],

            mirroring,

            buffer: 0x00,

            adr: PpuAdr::new(),
            ctr: PpuCtr::new(),
        }
    }

    pub fn write_adr(&mut self, data: u8) {
        self.adr.update(data);
    }

    pub fn write_ctr(&mut self, data: u8) {
        self.ctr.update(data);
    }

    fn increment_adr(&mut self) {
        self.adr.increment(self.ctr.get_adr_increment());
    }

    fn vram_mirror_adr(&self, adr: u16) -> u16 {
        let mirrored_adr = adr & 0x2fff;
        match (&self.mirroring, mirrored_adr) {
            (Horizontal, 0x2000..=0x27ff) => mirrored_adr & 0x03ff,
            (Horizontal, 0x2800..=0x2fff) => (mirrored_adr & 0x03ff) + 0x0400,
            (Vertical, 0x2000..=0x23ff | 0x2800..=0x2bff) => mirrored_adr & 0x03ff,
            (Vertical, 0x2400..=0x27ff | 0x2c00..=0x2fff) => (mirrored_adr & 0x03ff) + 0x0400,
            _ => panic!(
                "Mirroring type {:?} has not been implemented",
                self.mirroring
            ),
        }
    }

    pub fn read_data(&mut self) -> u8 {
        let adr = self.adr.address;
        self.increment_adr();

        match adr {
            0x0000..=0x1fff => {
                let res = self.buffer;
                self.buffer = self.chr_rom[adr as usize];
                res
            }
            0x2000..=0x2fff => {
                let res = self.buffer;
                self.buffer = self.vram[self.vram_mirror_adr(adr) as usize];
                res
            }
            0x3000..=0x3eff => panic!(
                "addresses in 0x3000..=0x3eff are not expected, requested: {}",
                adr
            ),
            0x3f00..=0x3fff => self.palette_table[(adr - 0x3f00) as usize],
            _ => panic!(
                "addresses in 0x4000..=0xffff are not expected, requested: {}",
                adr
            ),
        }
    }

    pub fn write_data(&mut self, data: u8) {
        let adr = self.adr.address;

        match adr {
            0x0000..=0x1fff => panic!("Attempted to write to chr rom at {:#x}", adr),
            0x2000..=0x2fff => self.vram[self.vram_mirror_adr(adr) as usize] = data,
            0x3000..=0x3eff => panic!(
                "addresses in 0x3000..=0x3eff are not expected, requested: {}", adr
            ),
            _ => panic!(
                "addresses in 0x4000..=0xffff are not expected, requested: {}", adr
            ),
        }
    }
}

pub struct PpuAdr {
    address: u16,
    hi_next: bool,
}

impl PpuAdr {
    pub fn new() -> Self {
        PpuAdr {
            address: 0x0000,
            hi_next: true,
        }
    }

    fn set(&mut self, address: u16) {
        self.address = address & 0x3fff;
    }

    /// Writes data to high or low byte of PPUADDR register
    pub fn update(&mut self, data: u8) {
        if self.hi_next {
            self.address &= 0x00ff;
            self.address |= (data as u16) << 8;
        } else {
            self.address &= 0xff00;
            self.address |= data as u16;
        }
        self.address &= 0x3fff;
        self.hi_next = !self.hi_next;
    }

    pub fn increment(&mut self, inc: u8) {
        self.address = self.address.wrapping_add(inc as u16) & 0x3fff;
    }

    pub fn reset_latch(&mut self) {
        self.hi_next = true;
    }
}

pub struct PpuCtr {
    flags: u8,
}

impl PpuCtr {
    // 7  bit  0
    // ---- ----
    // VPHB SINN
    // |||| ||||
    // |||| ||++- Base nametable address
    // |||| ||    (0 = $2000; 1 = $2400; 2 = $2800; 3 = $2C00)
    // |||| |+--- VRAM address increment per CPU read/write of PPUDATA
    // |||| |     (0: add 1, going across; 1: add 32, going down)
    // |||| +---- Sprite pattern table address for 8x8 sprites
    // ||||       (0: $0000; 1: $1000; ignored in 8x16 mode)
    // |||+------ Background pattern table address (0: $0000; 1: $1000)
    // ||+------- Sprite size (0: 8x8 pixels; 1: 8x16 pixels)
    // |+-------- PPU master/slave select
    // |          (0: read backdrop from EXT pins; 1: output color on EXT pins)
    // +--------- Generate an NMI at the start of the
    //            vertical blanking interval (0: off; 1: on)

    pub fn new() -> Self {
        PpuCtr { flags: 0 }
    }

    pub fn get_adr_increment(&self) -> u8 {
        if self.flags & 0b0000_0100 == 0 {
            1
        } else {
            32
        }
    }

    pub fn update(&mut self, data: u8) {
        self.flags = data;
    }
}

#[cfg(test)]
pub mod test {
    use super::*;

    fn test_ppu() -> PPU {
        PPU::new(vec![0; 0x0800], Horizontal)
    }

    #[test]
    fn test_ppu_vram_writes() {
        let mut ppu = test_ppu();
        ppu.write_adr(0x23);
        ppu.write_adr(0x05);
        ppu.write_data(0x66);

        assert_eq!(ppu.vram[0x0305], 0x66);
    }

    #[test]
    fn test_ppu_vram_reads() {
        let mut ppu = test_ppu();
        ppu.write_ctr(0);
        ppu.vram[0x0305] = 0x66;

        ppu.write_adr(0x23);
        ppu.write_adr(0x05);

        ppu.read_data(); //load_into_buffer
        assert_eq!(ppu.adr.address, 0x2306);
        assert_eq!(ppu.read_data(), 0x66);
    }

    #[test]
    fn test_ppu_vram_reads_cross_page() {
        let mut ppu = test_ppu();
        ppu.write_ctr(0);
        ppu.vram[0x01ff] = 0x66;
        ppu.vram[0x0200] = 0x77;

        ppu.write_adr(0x21);
        ppu.write_adr(0xff);

        ppu.read_data(); //load_into_buffer
        assert_eq!(ppu.read_data(), 0x66);
        assert_eq!(ppu.read_data(), 0x77);
    }

    #[test]
    fn test_ppu_vram_reads_step_32() {
        let mut ppu = test_ppu();
        ppu.write_ctr(0b100);
        ppu.vram[0x01ff] = 0x66;
        ppu.vram[0x01ff + 32] = 0x77;
        ppu.vram[0x01ff + 64] = 0x88;

        ppu.write_adr(0x21);
        ppu.write_adr(0xff);

        ppu.read_data(); //load_into_buffer
        assert_eq!(ppu.read_data(), 0x66);
        assert_eq!(ppu.read_data(), 0x77);
        assert_eq!(ppu.read_data(), 0x88);
    }

    #[test]
    fn test_vram_horizontal_mirror() {
        let mut ppu = test_ppu();
        ppu.write_adr(0x24);
        ppu.write_adr(0x05);

        ppu.write_data(0x66); //write to a

        ppu.write_adr(0x28);
        ppu.write_adr(0x05);

        ppu.write_data(0x77); //write to B

        ppu.write_adr(0x20);
        ppu.write_adr(0x05);

        ppu.read_data(); //load into buffer
        assert_eq!(ppu.read_data(), 0x66); //read from A

        ppu.write_adr(0x2C);
        ppu.write_adr(0x05);

        ppu.read_data(); //load into buffer
        assert_eq!(ppu.read_data(), 0x77); //read from b
    }

    #[test]
    fn test_vram_vertical_mirror() {
        let mut ppu = PPU::new(vec![0; 2048], Vertical);

        ppu.write_adr(0x20);
        ppu.write_adr(0x05);

        ppu.write_data(0x66); //write to A

        ppu.write_adr(0x2C);
        ppu.write_adr(0x05);

        ppu.write_data(0x77); //write to b

        ppu.write_adr(0x28);
        ppu.write_adr(0x05);

        ppu.read_data(); //load into buffer
        assert_eq!(ppu.read_data(), 0x66); //read from a

        ppu.write_adr(0x24);
        ppu.write_adr(0x05);

        ppu.read_data(); //load into buffer
        assert_eq!(ppu.read_data(), 0x77); //read from B
    }

    // #[test]
    // fn test_read_status_resets_latch() {
    //     let mut ppu = NesPPU::new_empty_rom();
    //     ppu.vram[0x0305] = 0x66;
    //
    //     ppu.write_to_ppu_addr(0x21);
    //     ppu.write_to_ppu_addr(0x23);
    //     ppu.write_to_ppu_addr(0x05);
    //
    //     ppu.read_data(); //load_into_buffer
    //     assert_ne!(ppu.read_data(), 0x66);
    //
    //     ppu.read_status();
    //
    //     ppu.write_to_ppu_addr(0x23);
    //     ppu.write_to_ppu_addr(0x05);
    //
    //     ppu.read_data(); //load_into_buffer
    //     assert_eq!(ppu.read_data(), 0x66);
    // }
    //
    // #[test]
    // fn test_ppu_vram_mirroring() {
    //     let mut ppu = NesPPU::new_empty_rom();
    //     ppu.write_to_ctrl(0);
    //     ppu.vram[0x0305] = 0x66;
    //
    //     ppu.write_to_ppu_addr(0x63); //0x6305 -> 0x2305
    //     ppu.write_to_ppu_addr(0x05);
    //
    //     ppu.read_data(); //load into_buffer
    //     assert_eq!(ppu.read_data(), 0x66);
    //     // assert_eq!(ppu.addr.read(), 0x0306)
    // }
    //
    // #[test]
    // fn test_read_status_resets_vblank() {
    //     let mut ppu = NesPPU::new_empty_rom();
    //     ppu.status.set_vblank_status(true);
    //
    //     let status = ppu.read_status();
    //
    //     assert_eq!(status >> 7, 1);
    //     assert_eq!(ppu.status.snapshot() >> 7, 0);
    // }
    //
    // #[test]
    // fn test_oam_read_write() {
    //     let mut ppu = NesPPU::new_empty_rom();
    //     ppu.write_to_oam_addr(0x10);
    //     ppu.write_to_oam_data(0x66);
    //     ppu.write_to_oam_data(0x77);
    //
    //     ppu.write_to_oam_addr(0x10);
    //     assert_eq!(ppu.read_oam_data(), 0x66);
    //
    //     ppu.write_to_oam_addr(0x11);
    //     assert_eq!(ppu.read_oam_data(), 0x77);
    // }
    //
    // #[test]
    // fn test_oam_dma() {
    //     let mut ppu = NesPPU::new_empty_rom();
    //
    //     let mut data = [0x66; 256];
    //     data[0] = 0x77;
    //     data[255] = 0x88;
    //
    //     ppu.write_to_oam_addr(0x10);
    //     ppu.write_oam_dma(&data);
    //
    //     ppu.write_to_oam_addr(0xf); //wrap around
    //     assert_eq!(ppu.read_oam_data(), 0x88);
    //
    //     ppu.write_to_oam_addr(0x10);
    //     assert_eq!(ppu.read_oam_data(), 0x77);
    //
    //     ppu.write_to_oam_addr(0x11);
    //     assert_eq!(ppu.read_oam_data(), 0x66);
    // }
}