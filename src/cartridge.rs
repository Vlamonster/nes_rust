#[derive(Debug)]
pub enum Mirroring {
    Vertical,
    Horizontal,
    FourScreen,
}

pub struct Rom {
    pub prg_rom: Vec<u8>,
    pub chr_rom: Vec<u8>,
    pub mapper_id: u8,
    pub screen_mirroring: Mirroring,
}

impl Rom {
    pub fn new(bytes: &[u8]) -> Rom {
        if bytes[0..4] != [0x4E, 0x45, 0x53, 0x1A] {
            panic!("File is not in iNES file format");
        }

        let mapper = (bytes[7] & 0b1111_0000) | (bytes[6] >> 4);

        if (bytes[7] >> 2) & 0b0000_0011 != 0 {
            panic!("NES2.0 format is not supported");
        }

        let screen_mirroring;
        if bytes[6] & 0b0000_1000 != 0 {
            screen_mirroring = Mirroring::FourScreen;
        } else if bytes[6] & 0b0000_0001 != 0 {
            screen_mirroring = Mirroring::Vertical;
        } else {
            screen_mirroring = Mirroring::Horizontal;
        }

        let prg_rom_size = bytes[4] as usize * 0x4000;
        let chr_rom_size = bytes[5] as usize * 0x2000;

        // check if rom contains a trainer so that we can skip it later
        let has_trainer = bytes[6] & 0b0000_0100 != 0;

        let prg_rom_start = 16 + if has_trainer { 512 } else { 0 };
        let chr_rom_start = prg_rom_start + prg_rom_size;

        Rom {
            prg_rom: bytes[prg_rom_start..(prg_rom_start + prg_rom_size)].to_vec(),
            chr_rom: bytes[chr_rom_start..(chr_rom_start + chr_rom_size)].to_vec(),
            mapper_id: mapper,
            screen_mirroring,
        }
    }
}

pub mod test {
    use super::*;

    struct TestRom {
        header: Vec<u8>,
        trainer: Option<Vec<u8>>,
        prg_rom: Vec<u8>,
        chr_rom: Vec<u8>,
    }

    fn create_rom(rom: TestRom) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(
            rom.header.len()
                + rom.trainer.as_ref().unwrap_or(&vec![]).len()
                + rom.prg_rom.len()
                + rom.chr_rom.len(),
        );

        bytes.extend(&rom.header);
        bytes.extend(rom.trainer.unwrap_or_default());
        bytes.extend(&rom.prg_rom);
        bytes.extend(&rom.chr_rom);

        bytes
    }

    /// Creates a simple nrom from the given program rom for testing
    pub fn test_rom(program: Vec<u8>) -> Rom {
        let test_rom = create_rom(TestRom {
            header: vec![
                0x4E, 0x45, 0x53, 0x1A, 0x02, 0x01, 0x31, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                0x00, 0x00,
            ],
            trainer: None,
            prg_rom: program,
            chr_rom: vec![0; 0x2000],
        });

        Rom::new(&test_rom)
    }
}
