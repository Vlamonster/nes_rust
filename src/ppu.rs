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

    pub register_control: PpuControl,
    pub register_mask: PpuMask,
    pub register_status: PpuStatus,
    pub oam_address: u8,
    pub register_scroll: PpuScroll,
    pub register_address: PpuAddress,
    // todo pub register_oam_dma: PpuOamDma,
}

impl PPU {
    pub fn new(chr_rom: Vec<u8>, mirroring: Mirroring) -> Self {
        PPU {
            chr_rom,
            vram: [0; 2048],
            oam_data: [0; 256],
            palette_table: [0; 32],

            mirroring,

            buffer: 0x00,

            register_control: PpuControl::new(),
            register_mask: PpuMask::new(),
            register_status: PpuStatus::new(),
            oam_address: 0x00,
            register_scroll: PpuScroll::new(),
            register_address: PpuAddress::new(),
            // todo register_oam_dma: PpuOamDma::new(),
        }
    }

    pub fn write_address(&mut self, data: u8) {
        self.register_address.update(data);
    }

    pub fn write_control(&mut self, data: u8) {
        self.register_control.update(data);
    }

    fn increment_adr(&mut self) {
        self.register_address
            .increment(self.register_control.get_address_increment());
    }

    fn vram_mirror_adr(&self, address: u16) -> u16 {
        let mirrored_adr = address & 0x2fff;
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
        let adr = self.register_address.address;
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
        let adr = self.register_address.address;

        match adr {
            0x0000..=0x1fff => panic!("Attempted to write to chr rom at {:#x}", adr),
            0x2000..=0x2fff => self.vram[self.vram_mirror_adr(adr) as usize] = data,
            0x3000..=0x3eff => panic!(
                "addresses in 0x3000..=0x3eff are not expected, requested: {}",
                adr
            ),
            _ => panic!(
                "addresses in 0x4000..=0xffff are not expected, requested: {}",
                adr
            ),
        }
    }

    pub fn read_status(&mut self) -> u8 {
        let data = self.register_status.flags;
        self.register_status.set_vertical_blank(false);
        self.register_address.reset_latch();
        self.register_scroll.reset_latch();
        data
    }

    pub fn write_oam_address(&mut self, data: u8){
        self.oam_address = data;
    }

    pub fn read_oam_data(&mut self) -> u8{
        self.oam_data[self.oam_address as usize]
    }

    pub fn write_oam_data(&mut self, data: u8){
        self.oam_data[self.oam_address as usize] = data;
        self.oam_address = self.oam_address.wrapping_add(1);
    }

    fn write_oam_dma(&mut self, data: &[u8; 256]) {
        for value in data.iter() {
            self.oam_data[self.oam_address as usize] = *value;
            self.oam_address = self.oam_address.wrapping_add(1);
        }
    }

    pub fn write_mask(&mut self, data: u8){
        self.register_mask.update(data);
    }

    pub fn write_scroll(&mut self, data: u8){
        self.register_scroll.update(data);
    }
}

pub struct PpuAddress {
    address: u16,
    hi_next: bool,
}

impl PpuAddress {
    pub fn new() -> Self {
        PpuAddress {
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

pub struct PpuControl {
    flags: u8,
}

impl PpuControl {
    pub fn new() -> Self {
        PpuControl { flags: 0x00 }
    }

    pub fn get_address_increment(&self) -> u8 {
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

pub struct PpuStatus {
    flags: u8,
}

impl PpuStatus {
    pub fn new() -> Self {
        PpuStatus { flags: 0x00 }
    }

    pub fn set_vertical_blank(&mut self, condition: bool) {
        if condition {
            self.flags |= 0b1000_0000;
        } else {
            self.flags &= !0b1000_0000;
        }
    }
}

pub struct PpuMask{
    flags: u8,
}

impl PpuMask{
    pub fn new() -> Self {
        PpuMask { flags: 0x00 }
    }

    pub fn update(&mut self, data: u8){
        self.flags = data;
    }
}

pub struct PpuScroll{
    pub x: u8,
    pub y: u8,
    pub x_next: bool,
}

impl PpuScroll{
    pub fn new() -> Self{
        PpuScroll{
            x: 0,
            y: 0,
            x_next: true,
        }
    }

    pub fn update(&mut self, data: u8){
        if self.x_next{
            self.x = data;
        } else {
            self.y = data;
        }
        self.x_next = !self.x_next;
    }

    pub fn reset_latch(&mut self){
        self.x_next = true;
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
        ppu.write_address(0x23);
        ppu.write_address(0x05);
        ppu.write_data(0x66);

        assert_eq!(ppu.vram[0x0305], 0x66);
    }

    #[test]
    fn test_ppu_vram_reads() {
        let mut ppu = test_ppu();
        ppu.write_control(0);
        ppu.vram[0x0305] = 0x66;

        ppu.write_address(0x23);
        ppu.write_address(0x05);

        ppu.read_data(); //load_into_buffer
        assert_eq!(ppu.register_address.address, 0x2306);
        assert_eq!(ppu.read_data(), 0x66);
    }

    #[test]
    fn test_ppu_vram_reads_cross_page() {
        let mut ppu = test_ppu();
        ppu.write_control(0);
        ppu.vram[0x01ff] = 0x66;
        ppu.vram[0x0200] = 0x77;

        ppu.write_address(0x21);
        ppu.write_address(0xff);

        ppu.read_data(); //load_into_buffer
        assert_eq!(ppu.read_data(), 0x66);
        assert_eq!(ppu.read_data(), 0x77);
    }

    #[test]
    fn test_ppu_vram_reads_step_32() {
        let mut ppu = test_ppu();
        ppu.write_control(0b100);
        ppu.vram[0x01ff] = 0x66;
        ppu.vram[0x01ff + 32] = 0x77;
        ppu.vram[0x01ff + 64] = 0x88;

        ppu.write_address(0x21);
        ppu.write_address(0xff);

        ppu.read_data(); //load_into_buffer
        assert_eq!(ppu.read_data(), 0x66);
        assert_eq!(ppu.read_data(), 0x77);
        assert_eq!(ppu.read_data(), 0x88);
    }

    #[test]
    fn test_vram_horizontal_mirror() {
        let mut ppu = test_ppu();
        ppu.write_address(0x24);
        ppu.write_address(0x05);

        ppu.write_data(0x66); //write to a

        ppu.write_address(0x28);
        ppu.write_address(0x05);

        ppu.write_data(0x77); //write to B

        ppu.write_address(0x20);
        ppu.write_address(0x05);

        ppu.read_data(); //load into buffer
        assert_eq!(ppu.read_data(), 0x66); //read from A

        ppu.write_address(0x2C);
        ppu.write_address(0x05);

        ppu.read_data(); //load into buffer
        assert_eq!(ppu.read_data(), 0x77); //read from b
    }

    #[test]
    fn test_vram_vertical_mirror() {
        let mut ppu = PPU::new(vec![0; 2048], Vertical);

        ppu.write_address(0x20);
        ppu.write_address(0x05);

        ppu.write_data(0x66); //write to A

        ppu.write_address(0x2C);
        ppu.write_address(0x05);

        ppu.write_data(0x77); //write to b

        ppu.write_address(0x28);
        ppu.write_address(0x05);

        ppu.read_data(); //load into buffer
        assert_eq!(ppu.read_data(), 0x66); //read from a

        ppu.write_address(0x24);
        ppu.write_address(0x05);

        ppu.read_data(); //load into buffer
        assert_eq!(ppu.read_data(), 0x77); //read from B
    }

    #[test]
    fn test_read_status_resets_latch() {
        let mut ppu = test_ppu();
        ppu.vram[0x0305] = 0x66;

        ppu.write_address(0x21);
        ppu.write_address(0x23);
        ppu.write_address(0x05);

        ppu.read_data(); //load_into_buffer
        assert_ne!(ppu.read_data(), 0x66);

        ppu.read_status();

        ppu.write_address(0x23);
        ppu.write_address(0x05);

        ppu.read_data(); //load_into_buffer
        assert_eq!(ppu.read_data(), 0x66);
    }

    #[test]
    fn test_ppu_vram_mirroring() {
        let mut ppu = test_ppu();
        ppu.write_control(0);
        ppu.vram[0x0305] = 0x66;

        ppu.write_address(0x63); //0x6305 -> 0x2305
        ppu.write_address(0x05);

        ppu.read_data(); //load into_buffer
        assert_eq!(ppu.read_data(), 0x66);
    }

    #[test]
    fn test_read_status_resets_vertical_blank() {
        let mut ppu = test_ppu();
        ppu.register_status.set_vertical_blank(true);

        let status = ppu.read_status();

        assert_eq!(status >> 7, 1);
        assert_eq!(ppu.register_status.flags >> 7, 0);
    }

    #[test]
    fn test_oam_read_write() {
        let mut ppu = test_ppu();
        ppu.write_oam_address(0x10);
        ppu.write_oam_data(0x66);
        ppu.write_oam_data(0x77);

        ppu.write_oam_address(0x10);
        assert_eq!(ppu.read_oam_data(), 0x66);

        ppu.write_oam_address(0x11);
        assert_eq!(ppu.read_oam_data(), 0x77);
    }

    #[test]
    fn test_oam_dma() {
        let mut ppu = test_ppu();

        let mut data = [0x66; 256];
        data[0] = 0x77;
        data[255] = 0x88;

        ppu.write_oam_address(0x10);
        ppu.write_oam_dma(&data);

        ppu.write_oam_address(0xf); //wrap around
        assert_eq!(ppu.read_oam_data(), 0x88);

        ppu.write_oam_address(0x10);
        assert_eq!(ppu.read_oam_data(), 0x77);

        ppu.write_oam_address(0x11);
        assert_eq!(ppu.read_oam_data(), 0x66);
    }
}
