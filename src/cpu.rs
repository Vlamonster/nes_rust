use crate::cpu::AddressingMode::{
    Absolute, AbsoluteX, AbsoluteY, Immediate, Implied, Indirect, IndirectX, IndirectY, ZeroPage,
    ZeroPageX, ZeroPageY,
};
use crate::opcodes;
use std::collections::HashMap;

// status register bits, useful for dealing with flags
const FLG_C: u8 = 0b0000_0001;
const FLG_Z: u8 = 0b0000_0010;
const FLG_I: u8 = 0b0000_0100;
const FLG_D: u8 = 0b0000_1000;
const FLG_B: u8 = 0b0001_0000;
const FLG_U: u8 = 0b0010_0000;
const FLG_V: u8 = 0b0100_0000;
const FLG_N: u8 = 0b1000_0000;

/// Struct of the CPU, which contains all the registers and 64KB memory.
pub struct CPU {
    pub a: u8,
    pub x: u8,
    pub y: u8,
    pub p: u8,
    pub s: u8,
    pub pc: u16,
    mem: [u8; 0x10000],
}

#[derive(Debug)]
/// Enum of all possible addressing modes.
pub enum AddressingMode {
    Immediate,
    ZeroPage,
    ZeroPageX,
    ZeroPageY,
    Absolute,
    AbsoluteX,
    AbsoluteY,
    Indirect,
    IndirectX,
    IndirectY,
    Implied,
}

/// Trait that allows reading and writing 1 or 2 bytes from device.
trait Mem {
    fn read(&self, adr: u16) -> u8;

    fn write(&mut self, adr: u16, val: u8);

    fn read_address(&self, adr: u16) -> u16 {
        let lo = self.read(adr) as u16;
        let hi = self.read(adr + 1) as u16;
        (hi << 8) | (lo as u16)
    }

    fn write_address(&mut self, adr: u16, val: u16) {
        let hi = (val >> 8) as u8;
        let lo = (val & 0xff) as u8;
        self.write(adr, lo);
        self.write(adr + 1, hi);
    }
}

impl Mem for CPU {
    fn read(&self, adr: u16) -> u8 {
        self.mem[adr as usize]
    }

    fn write(&mut self, adr: u16, val: u8) {
        self.mem[adr as usize] = val;
    }
}

impl CPU {
    pub fn new() -> Self {
        CPU {
            a: 0,
            x: 0,
            y: 0,
            p: 0,
            s: 0,
            pc: 0,
            mem: [0; 0x10000],
        }
    }

    fn get_operand_address(&self, mode: &AddressingMode) -> u16 {
        match mode {
            Immediate => self.pc,
            ZeroPage => self.read(self.pc) as u16,
            Absolute => self.read_address(self.pc),
            ZeroPageX => {
                let pos = self.read(self.pc);
                let addr = pos.wrapping_add(self.x) as u16;
                addr
            }
            ZeroPageY => {
                let pos = self.read(self.pc);
                let addr = pos.wrapping_add(self.y) as u16;
                addr
            }

            AbsoluteX => {
                let base = self.read_address(self.pc);
                let addr = base.wrapping_add(self.x as u16);
                addr
            }
            AbsoluteY => {
                let base = self.read_address(self.pc);
                let addr = base.wrapping_add(self.y as u16);
                addr
            }
            Indirect => {
                let ptr = self.read_address(self.pc);

                // An original 6502 has does not correctly fetch the target address
                // if the indirect vector falls on a page boundary
                let lo = self.read(ptr);
                let hi = if ptr & 0xff == 0xff {
                    self.read(ptr & 0xff00)
                } else {
                    self.read(ptr.wrapping_add(1))
                };
                (hi as u16) << 8 | (lo as u16)
            }
            IndirectX => {
                let base = self.read(self.pc);

                let ptr: u8 = (base as u8).wrapping_add(self.x);
                let lo = self.read(ptr as u16);
                let hi = self.read(ptr.wrapping_add(1) as u16);
                (hi as u16) << 8 | (lo as u16)
            }
            IndirectY => {
                let base = self.read(self.pc);

                let lo = self.read(base as u16);
                let hi = self.read((base as u8).wrapping_add(1) as u16);
                let deref_base = (hi as u16) << 8 | (lo as u16);
                let deref = deref_base.wrapping_add(self.y as u16);
                deref
            }

            Implied => {
                panic!("mode {:?} is not supported", mode);
            }
        }
    }

    fn update_flag(&mut self, flag: u8, condition: bool) {
        if condition {
            self.p |= flag;
        } else {
            self.p &= !flag;
        }
    }

    fn update_zn_flags(&mut self, result: u8) {
        self.update_flag(FLG_Z, result == 0);
        self.update_flag(FLG_N, result & FLG_N != 0);
    }

    fn stack_push(&mut self, value: u8) {
        self.write(0x0100 | self.s as u16, value);
        self.s = self.s.wrapping_sub(1);
    }

    fn stack_pop(&mut self) -> u8 {
        self.s = self.s.wrapping_add(1);
        self.read(0x0100 | self.s as u16)
    }

    pub fn load_and_run(&mut self, program: Vec<u8>, timeout: bool, max_time: u64) {
        self.load(program);
        self.reset();
        self.run(timeout, max_time);
    }

    pub fn load(&mut self, program: Vec<u8>) {
        self.mem[0x8000..(0x8000 + program.len())].copy_from_slice(&program[..]);
        self.write_address(0xFFFC, 0x8000);
    }

    pub fn reset(&mut self) {
        self.a = 0;
        self.x = 0;
        self.y = 0;
        self.p = 0;
        self.s = 0xfd;

        self.pc = self.read_address(0xFFFC);
    }

    pub fn run(&mut self, timeout: bool, max_time: u64) {
        let ref opcodes: HashMap<u8, &'static opcodes::OpCode> = *opcodes::OPCODES_MAP;

        let mut run_time = max_time;

        while !timeout || run_time > 0 {
            let code = self.read(self.pc);
            self.pc += 1;
            let program_counter_state = self.pc;

            let opcode = opcodes
                .get(&code)
                .expect(&format!("OpCode {:x} is not recognized", code));

            run_time = run_time.wrapping_sub(opcode.len as u64);

            match code {
                0x69 | 0x65 | 0x75 | 0x6d | 0x7d | 0x79 | 0x61 | 0x71 => self.adc(&opcode.mode),
                0x29 | 0x25 | 0x35 | 0x2d | 0x3d | 0x39 | 0x21 | 0x31 => self.and(&opcode.mode),
                0x0a | 0x06 | 0x16 | 0x0e | 0x1e => self.asl(&opcode.mode),
                0x90 => self.bcc(),
                0xb0 => self.bcs(),
                0xf0 => self.beq(),
                0x24 | 0x2c => self.bit(&opcode.mode),
                0x30 => self.bmi(),
                0xd0 => self.bne(),
                0x10 => self.bpl(),
                0x00 => self.brk(),
                0x50 => self.bvc(),
                0x70 => self.bvs(),
                0x18 => self.clc(),
                0xd8 => self.cld(),
                0x58 => self.cli(),
                0xb8 => self.clv(),
                0xc9 | 0xc5 | 0xd5 | 0xcd | 0xdd | 0xd9 | 0xc1 | 0xd1 => self.cmp(&opcode.mode),
                0xe0 | 0xe4 | 0xec => self.cpx(&opcode.mode),
                0xc0 | 0xc4 | 0xcc => self.cpy(&opcode.mode),
                0xc6 | 0xd6 | 0xce | 0xde => self.dec(&opcode.mode),
                0xca => self.dex(),
                0x88 => self.dey(),
                0x49 | 0x45 | 0x55 | 0x4d | 0x5d | 0x59 | 0x41 | 0x51 => self.eor(&opcode.mode),
                0xe6 | 0xf6 | 0xee | 0xfe => self.inc(&opcode.mode),
                0xe8 => self.inx(),
                0xc8 => self.iny(),
                0x4c | 0x6c => self.jmp(&opcode.mode),
                0x20 => self.jsr(&opcode.mode),
                0xa9 | 0xa5 | 0xb5 | 0xad | 0xbd | 0xb9 | 0xa1 | 0xb1 => self.lda(&opcode.mode),
                0xa2 | 0xa6 | 0xb6 | 0xae | 0xbe => self.ldx(&opcode.mode),
                0xa0 | 0xa4 | 0xb4 | 0xac | 0xbc => self.ldy(&opcode.mode),
                0x4a | 0x46 | 0x56 | 0x4e | 0x5e => self.lsr(&opcode.mode),
                0xea => self.nop(),
                0x09 | 0x05 | 0x15 | 0x0d | 0x1d | 0x19 | 0x01 | 0x11 => self.ora(&opcode.mode),
                0x48 => self.pha(),
                0x08 => self.php(),
                0x68 => self.pla(),
                0x28 => self.plp(),
                0x2a | 0x26 | 0x36 | 0x2e | 0x3e => self.rol(&opcode.mode),
                0x6a | 0x66 | 0x76 | 0x6e | 0x7e => self.ror(&opcode.mode),
                0x40 => self.rti(),
                0x60 => self.rts(),
                0xe9 | 0xe5 | 0xf5 | 0xed | 0xfd | 0xf9 | 0xe1 | 0xf1 => self.sbc(&opcode.mode),
                0x38 => self.sec(),
                0xf8 => self.sed(),
                0x78 => self.sei(),
                0x85 | 0x95 | 0x8d | 0x9d | 0x99 | 0x81 | 0x91 => self.sta(&opcode.mode),
                0x86 | 0x96 | 0x8e => self.stx(&opcode.mode),
                0x84 | 0x94 | 0x8c => self.sty(&opcode.mode),
                0xaa => self.tax(),
                0xa8 => self.tay(),
                0xba => self.tsx(),
                0x8a => self.txa(),
                0x9a => self.txs(),
                0x98 => self.tya(),
                _ => todo!("OpCode was parsed, but has not been implemented yet."),
            }

            if program_counter_state == self.pc {
                self.pc += (opcode.len - 1) as u16;
            }
        }
    }

    fn adc(&mut self, mode: &AddressingMode) {
        let adr = self.get_operand_address(mode);
        let val = self.read(adr);

        let (tmp, c1) = self.a.overflowing_add(val);
        let (res, c2) = tmp.overflowing_add(self.p & 0x01);

        self.update_zn_flags(res);
        self.update_flag(FLG_C, c1 || c2);
        self.update_flag(FLG_V, (self.a ^ res) & (val ^ res) & FLG_N != 0);

        self.a = res;
    }

    fn and(&mut self, mode: &AddressingMode) {
        let adr = self.get_operand_address(mode);
        let val = self.read(adr);

        self.a &= val;
        self.update_zn_flags(self.a);
    }

    fn asl(&mut self, mode: &AddressingMode) {
        match mode {
            Implied => {
                self.update_flag(FLG_C, self.a & 0b1000_0000 != 0);

                self.a <<= 1;
                self.update_zn_flags(self.a);
            }
            _ => {
                let adr = self.get_operand_address(mode);
                let val = self.read(adr);

                let res = val << 1;

                self.update_flag(FLG_C, val & 0b1000_0000 != 0);
                self.update_zn_flags(res);
                self.write(adr, res);
            }
        }
    }

    fn bcc(&mut self) {
        if self.p & FLG_C == 0 {
            let offset = self.read(self.pc) as i8;
            self.pc = ((self.pc as i16) + offset as i16 + 1) as u16;
        }
    }

    fn bcs(&mut self) {
        if self.p & FLG_C != 0 {
            let offset = self.read(self.pc) as i8;
            self.pc = ((self.pc as i16) + offset as i16 + 1) as u16;
        }
    }

    fn beq(&mut self) {
        if self.p & FLG_Z != 0 {
            let offset = self.read(self.pc) as i8;
            self.pc = ((self.pc as i16) + offset as i16 + 1) as u16;
        }
    }

    fn bit(&mut self, mode: &AddressingMode) {
        let adr = self.get_operand_address(mode);
        let val = self.read(adr);

        self.update_flag(FLG_Z, self.a & val == 0);
        self.update_flag(FLG_V, val & FLG_V != 0);
        self.update_flag(FLG_N, val & FLG_N != 0);
    }

    fn bmi(&mut self) {
        if self.p & FLG_N != 0 {
            let offset = self.read(self.pc) as i8;
            self.pc = ((self.pc as i16) + offset as i16 + 1) as u16;
        }
    }

    fn bne(&mut self) {
        if self.p & FLG_Z == 0 {
            let offset = self.read(self.pc) as i8;
            self.pc = ((self.pc as i16) + offset as i16 + 1) as u16;
        }
    }

    fn bpl(&mut self) {
        if self.p & FLG_N == 0 {
            let offset = self.read(self.pc) as i8;
            self.pc = ((self.pc as i16) + offset as i16 + 1) as u16;
        }
    }

    fn brk(&mut self) {
        self.stack_push((self.pc >> 8) as u8);
        self.stack_push((self.pc & 0xff) as u8);
        self.stack_push(self.p | FLG_U | FLG_B);

        self.update_flag(FLG_I, true);

        self.pc = self.read_address(0xfffe);
    }

    fn bvc(&mut self) {
        if self.p & FLG_V == 0 {
            let offset = self.read(self.pc) as i8;
            self.pc = ((self.pc as i16) + offset as i16 + 1) as u16;
        }
    }

    fn bvs(&mut self) {
        if self.p & FLG_V != 0 {
            let offset = self.read(self.pc) as i8;
            self.pc = ((self.pc as i16) + offset as i16 + 1) as u16;
        }
    }

    fn clc(&mut self) {
        self.update_flag(FLG_C, false);
    }

    fn cld(&mut self) {
        self.update_flag(FLG_D, false);
    }

    fn cli(&mut self) {
        self.update_flag(FLG_I, false);
    }

    fn clv(&mut self) {
        self.update_flag(FLG_V, false);
    }

    fn cmp(&mut self, mode: &AddressingMode) {
        let adr = self.get_operand_address(&mode);
        let val = self.read(adr);

        self.update_flag(FLG_C, self.a >= val);
        self.update_flag(FLG_Z, self.a == val);
        self.update_flag(FLG_N, self.a.wrapping_sub(val) & FLG_N != 0);
    }

    fn cpx(&mut self, mode: &AddressingMode) {
        let adr = self.get_operand_address(&mode);
        let val = self.read(adr);

        self.update_flag(FLG_C, self.x >= val);
        self.update_flag(FLG_Z, self.x == val);
        self.update_flag(FLG_N, self.x.wrapping_sub(val) & FLG_N != 0);
    }

    fn cpy(&mut self, mode: &AddressingMode) {
        let adr = self.get_operand_address(&mode);
        let val = self.read(adr);

        self.update_flag(FLG_C, self.y >= val);
        self.update_flag(FLG_Z, self.y == val);
        self.update_flag(FLG_N, self.y.wrapping_sub(val) & FLG_N != 0);
    }

    fn dec(&mut self, mode: &AddressingMode) {
        let adr = self.get_operand_address(&mode);
        let val = self.read(adr);

        let res = val.wrapping_sub(1);

        self.write(adr, res);
        self.update_zn_flags(res);
    }

    fn dex(&mut self) {
        self.x = self.x.wrapping_sub(1);
        self.update_zn_flags(self.x);
    }

    fn dey(&mut self) {
        self.y = self.y.wrapping_sub(1);
        self.update_zn_flags(self.y);
    }

    fn eor(&mut self, mode: &AddressingMode) {
        let adr = self.get_operand_address(&mode);
        let val = self.read(adr);

        self.a ^= val;
        self.update_zn_flags(self.a);
    }

    fn inc(&mut self, mode: &AddressingMode) {
        let adr = self.get_operand_address(&mode);
        let val = self.read(adr);

        let res = val.wrapping_add(1);

        self.write(adr, res);
        self.update_zn_flags(res);
    }

    fn inx(&mut self) {
        self.x = self.x.wrapping_add(1);
        self.update_zn_flags(self.x);
    }

    fn iny(&mut self) {
        self.y = self.y.wrapping_add(1);
        self.update_zn_flags(self.y);
    }

    fn jmp(&mut self, mode: &AddressingMode) {
        let adr = self.get_operand_address(&mode);
        self.pc = adr;
    }

    fn jsr(&mut self, mode: &AddressingMode) {
        let adr = self.get_operand_address(&mode);

        self.stack_push(((self.pc + 1) >> 8) as u8);
        self.stack_push(((self.pc + 1) & 0x00ff) as u8);

        self.pc = adr;
    }

    fn lda(&mut self, mode: &AddressingMode) {
        let adr = self.get_operand_address(&mode);
        let val = self.read(adr);

        self.a = val;
        self.update_zn_flags(self.a);
    }

    fn ldx(&mut self, mode: &AddressingMode) {
        let adr = self.get_operand_address(&mode);
        let val = self.read(adr);

        self.x = val;
        self.update_zn_flags(self.x);
    }

    fn ldy(&mut self, mode: &AddressingMode) {
        let adr = self.get_operand_address(&mode);
        let val = self.read(adr);

        self.y = val;
        self.update_zn_flags(self.y);
    }

    fn lsr(&mut self, mode: &AddressingMode) {
        match mode {
            Implied => {
                self.update_flag(FLG_C, self.a & 0b0000_0001 != 0);

                self.a >>= 1;
                self.update_zn_flags(self.a);
            }
            _ => {
                let adr = self.get_operand_address(mode);
                let val = self.read(adr);

                let res = val >> 1;

                self.update_flag(FLG_C, val & 0b0000_0001 != 0);
                self.update_zn_flags(res);
                self.write(adr, res);
            }
        }
    }

    fn nop(&mut self) {}

    fn ora(&mut self, mode: &AddressingMode) {
        let adr = self.get_operand_address(&mode);
        let val = self.read(adr);

        self.a |= val;
        self.update_zn_flags(self.a);
    }

    fn pha(&mut self) {
        self.stack_push(self.a);
    }

    fn php(&mut self) {
        self.stack_push(self.p | FLG_U | FLG_B);
    }

    fn pla(&mut self) {
        self.a = self.stack_pop();
        self.update_zn_flags(self.a);
    }

    fn plp(&mut self) {
        self.p = self.stack_pop() & !FLG_U & !FLG_B;
    }

    fn rol(&mut self, mode: &AddressingMode) {
        match mode {
            Implied => {
                let flg_c = self.p & FLG_C;
                self.update_flag(FLG_C, self.a & 0b1000_0000 != 0);

                self.a <<= 1;
                self.a |= flg_c;
                self.update_zn_flags(self.a);
            }
            _ => {
                let adr = self.get_operand_address(mode);
                let val = self.read(adr);

                let res = val << 1 | (self.p & FLG_C);

                self.update_flag(FLG_C, val & 0b1000_0000 != 0);
                self.update_zn_flags(res);
                self.write(adr, res);
            }
        }
    }

    fn ror(&mut self, mode: &AddressingMode) {
        match mode {
            Implied => {
                let flg_c = self.p & FLG_C;
                self.update_flag(FLG_C, self.a & 0b0000_0001 != 0);

                self.a >>= 1;
                self.a |= flg_c << 7;
                self.update_zn_flags(self.a);
            }
            _ => {
                let adr = self.get_operand_address(mode);
                let val = self.read(adr);

                let res = val >> 1 | (self.p & FLG_C) << 7;

                self.update_flag(FLG_C, val & 0b0000_0001 != 0);
                self.update_zn_flags(res);
                self.write(adr, res);
            }
        }
    }

    fn rti(&mut self) {
        self.p = self.stack_pop() & !FLG_U & !FLG_B;
        self.pc = self.stack_pop() as u16 | (self.stack_pop() as u16) << 8;
    }

    fn rts(&mut self) {
        self.pc = (self.stack_pop() as u16 | (self.stack_pop() as u16) << 8) + 1;
    }

    fn sbc(&mut self, mode: &AddressingMode) {
        let adr = self.get_operand_address(mode);
        let val = (!self.read(adr)).wrapping_add(1); // might not need +1

        let (tmp, c1) = self.a.overflowing_add(val);
        let (res, c2) = tmp.overflowing_add(self.p & 0x01);

        self.update_zn_flags(res);
        self.update_flag(FLG_C, c1 || c2);
        self.update_flag(FLG_V, (self.a ^ res) & (val ^ res) & FLG_N != 0);

        self.a = res;
    }

    fn sec(&mut self) {
        self.update_flag(FLG_C, true);
    }

    fn sed(&mut self) {
        self.update_flag(FLG_D, true);
    }

    fn sei(&mut self) {
        self.update_flag(FLG_I, true);
    }

    fn sta(&mut self, mode: &AddressingMode) {
        let adr = self.get_operand_address(mode);
        self.write(adr, self.a);
    }

    fn stx(&mut self, mode: &AddressingMode) {
        let adr = self.get_operand_address(mode);
        self.write(adr, self.x);
    }

    fn sty(&mut self, mode: &AddressingMode) {
        let adr = self.get_operand_address(mode);

        self.write(adr, self.y);
    }

    fn tax(&mut self) {
        self.x = self.a;
        self.update_zn_flags(self.x);
    }

    fn tay(&mut self) {
        self.y = self.a;
        self.update_zn_flags(self.y);
    }

    fn tsx(&mut self) {
        self.x = self.s;
        self.update_zn_flags(self.x);
    }

    fn txa(&mut self) {
        self.a = self.x;
        self.update_zn_flags(self.a);
    }

    fn txs(&mut self) {
        self.s = self.x;
    }

    fn tya(&mut self) {
        self.a = self.y;
        self.update_zn_flags(self.a);
    }
}

#[cfg(test)]
mod test {
    use super::*;

    fn test_cpu(cpu: &mut CPU, program: Vec<u8>) {
        let max_time = program.len() as u64;
        cpu.load_and_run(program, true, max_time);
    }

    #[test]
    fn test_adc() {
        let mut cpu = CPU::new();
        test_cpu(&mut cpu, vec![0xa9, 0x05, 0x69, 0x10]);
        assert_eq!(cpu.a, 0x15);
    }

    #[test]
    fn test_and() {
        let mut cpu = CPU::new();
        test_cpu(&mut cpu, vec![0xa9, 0xf0, 0x29, 0x8f]);
        assert_eq!(cpu.a, 0x80);
    }

    #[test]
    fn test_asl() {
        let mut cpu = CPU::new();
        test_cpu(&mut cpu, vec![0xa9, 0xf0, 0x0a]);
        assert_eq!(cpu.a, 0xe0);
    }

    #[test]
    fn test_bcc() {
        let mut cpu = CPU::new();
        test_cpu(&mut cpu, vec![0x90, 0x02, 0xa9, 0xff]);
        assert_eq!(cpu.a, 0x00);
    }

    #[test]
    fn test_bcs() {
        let mut cpu = CPU::new();
        test_cpu(&mut cpu, vec![0xb0, 0x01, 0x00, 0xa9, 0xff]);
        assert_eq!(cpu.a, 0x00);
    }

    #[test]
    fn test_beq() {
        let mut cpu = CPU::new();
        test_cpu(&mut cpu, vec![0xf0, 0x01, 0x00, 0xa9, 0xff]);
        assert_eq!(cpu.a, 0x00);
    }

    #[test]
    fn test_bit() {
        let mut cpu = CPU::new();
        cpu.write(0x0000, 0xf0);
        test_cpu(&mut cpu, vec![0x2c, 0x00, 0x00]);
        assert_ne!(cpu.p & FLG_Z, 0);
        assert_ne!(cpu.p & FLG_V, 0);
        assert_ne!(cpu.p & FLG_N, 0);
    }

    #[test]
    fn test_bmi() {
        let mut cpu = CPU::new();
        test_cpu(&mut cpu, vec![0x30, 0x01, 0x00, 0xa9, 0xff]);
        assert_eq!(cpu.a, 0x00);
    }

    #[test]
    fn test_bne() {
        let mut cpu = CPU::new();
        test_cpu(&mut cpu, vec![0xd0, 0x02, 0xa9, 0xff]);
        assert_eq!(cpu.a, 0x00);
    }

    #[test]
    fn test_bpl() {
        let mut cpu = CPU::new();
        test_cpu(&mut cpu, vec![0x10, 0x02, 0xa9, 0xff]);
        assert_eq!(cpu.a, 0x00);
    }

    // todo add BRK test
    #[test]
    fn test_bvc() {
        let mut cpu = CPU::new();
        test_cpu(&mut cpu, vec![0x50, 0x02, 0xa9, 0xff]);
        assert_eq!(cpu.a, 0x00);
    }

    #[test]
    fn test_bvs() {
        let mut cpu = CPU::new();
        test_cpu(&mut cpu, vec![0x70, 0x01, 0x00, 0xa9, 0xff]);
        assert_eq!(cpu.a, 0x00);
    }

    #[test]
    fn test_clc() {
        let mut cpu = CPU::new();
        test_cpu(&mut cpu, vec![0x38, 0x18]);
        assert_eq!(cpu.p & FLG_C, 0)
    }

    #[test]
    fn test_cld() {
        let mut cpu = CPU::new();
        test_cpu(&mut cpu, vec![0xf8, 0xd8]);
        assert_eq!(cpu.p & FLG_D, 0)
    }

    #[test]
    fn test_cli() {
        let mut cpu = CPU::new();
        test_cpu(&mut cpu, vec![0x78, 0x58]);
        assert_eq!(cpu.p & FLG_I, 0)
    }

    #[test]
    fn test_clv() {
        let mut cpu = CPU::new();
        test_cpu(&mut cpu, vec![0x69, 0x80, 0x69, 0x80, 0xb8]);
        assert_eq!(cpu.p & FLG_V, 0)
    }

    #[test]
    fn test_cmp() {
        let mut cpu = CPU::new();
        test_cpu(&mut cpu, vec![0xc9, 0x00]);
        assert_ne!(cpu.p & FLG_C, 0);
        assert_ne!(cpu.p & FLG_Z, 0);
        assert_eq!(cpu.p & FLG_V, 0);
    }

    #[test]
    fn test_cpx() {
        let mut cpu = CPU::new();
        test_cpu(&mut cpu, vec![0xe0, 0x00]);
        assert_ne!(cpu.p & FLG_C, 0);
        assert_ne!(cpu.p & FLG_Z, 0);
        assert_eq!(cpu.p & FLG_V, 0);
    }

    #[test]
    fn test_cpy() {
        let mut cpu = CPU::new();
        test_cpu(&mut cpu, vec![0xc0, 0x00]);
        assert_ne!(cpu.p & FLG_C, 0);
        assert_ne!(cpu.p & FLG_Z, 0);
        assert_eq!(cpu.p & FLG_V, 0);
    }

    #[test]
    fn test_dec() {
        let mut cpu = CPU::new();
        test_cpu(&mut cpu, vec![0xce, 0x00, 0x00]);
        assert_eq!(cpu.read(0x0000), 0xff);
    }

    #[test]
    fn test_dex() {
        let mut cpu = CPU::new();
        test_cpu(&mut cpu, vec![0xca]);
        assert_eq!(cpu.x, 0xff);
    }

    #[test]
    fn test_dey() {
        let mut cpu = CPU::new();
        test_cpu(&mut cpu, vec![0x88]);
        assert_eq!(cpu.y, 0xff);
    }

    #[test]
    fn test_eor() {
        let mut cpu = CPU::new();
        test_cpu(&mut cpu, vec![0xa9, 0xff, 0x49, 0xf0]);
        assert_eq!(cpu.a, 0x0f);
    }

    #[test]
    fn test_inc() {
        let mut cpu = CPU::new();
        test_cpu(&mut cpu, vec![0xee, 0x00, 0x00]);
        assert_eq!(cpu.read(0x0000), 0x01);
    }

    #[test]
    fn test_inx() {
        let mut cpu = CPU::new();
        test_cpu(&mut cpu, vec![0xe8]);
        assert_eq!(cpu.x, 0x01);
    }

    #[test]
    fn test_iny() {
        let mut cpu = CPU::new();
        test_cpu(&mut cpu, vec![0xc8]);
        assert_eq!(cpu.y, 0x01);
    }

    #[test]
    fn test_jmp() {
        let mut cpu = CPU::new();
        test_cpu(&mut cpu, vec![0x4c, 0xaa, 0xbb]);
        assert_eq!(cpu.pc, 0xbbaa);
    }

    #[test]
    fn test_jsr() {
        let mut cpu = CPU::new();
        test_cpu(&mut cpu, vec![0x20, 0xaa, 0xbb]);
        assert_eq!(cpu.pc, 0xbbaa);
        assert_eq!(cpu.stack_pop(), 0x00 + 2);
        assert_eq!(cpu.stack_pop(), 0x80);
    }

    #[test]
    fn test_lda() {
        let mut cpu = CPU::new();
        test_cpu(&mut cpu, vec![0xa9, 0xee]);
        assert_eq!(cpu.a, 0xee);
    }

    #[test]
    fn test_ldx() {
        let mut cpu = CPU::new();
        test_cpu(&mut cpu, vec![0xa2, 0xee]);
        assert_eq!(cpu.x, 0xee);
    }

    #[test]
    fn test_ldy() {
        let mut cpu = CPU::new();
        test_cpu(&mut cpu, vec![0xa0, 0xee]);
        assert_eq!(cpu.y, 0xee);
    }

    #[test]
    fn test_lsr() {
        let mut cpu = CPU::new();
        test_cpu(&mut cpu, vec![0xa9, 0x0f, 0x4a]);
        assert_eq!(cpu.a, 0x07);
        assert_ne!(cpu.p & FLG_C, 0)
    }

    #[test]
    fn test_nop() {
        let mut cpu = CPU::new();
        test_cpu(&mut cpu, vec![0xa9, 0xcd, 0xea]);
        assert_eq!(cpu.a, 0xcd);
    }

    #[test]
    fn test_ora() {
        let mut cpu = CPU::new();
        test_cpu(&mut cpu, vec![0x09, 0xf0]);
        assert_eq!(cpu.a, 0xf0);
    }

    #[test]
    fn test_pha() {
        let mut cpu = CPU::new();
        test_cpu(&mut cpu, vec![0xa9, 0xf0, 0x48]);
        assert_ne!(cpu.stack_pop() & FLG_N, 0);
    }

    #[test]
    fn test_php() {
        let mut cpu = CPU::new();
        test_cpu(&mut cpu, vec![0x78, 0x08]);
        assert_ne!(cpu.stack_pop() & FLG_I, 0);
    }

    #[test]
    fn test_pla() {
        let mut cpu = CPU::new();
        test_cpu(&mut cpu, vec![0xa9, 0xf0, 0x48, 0xa9, 0x00, 0x68]);
        assert_eq!(cpu.a, 0xf0);
    }

    #[test]
    fn test_plp() {
        let mut cpu = CPU::new();
        test_cpu(&mut cpu, vec![0x08, 0x78, 0x28]);
        assert_eq!(cpu.p, 0);
    }

    #[test]
    fn test_rol() {
        let mut cpu = CPU::new();
        test_cpu(&mut cpu, vec![0x38, 0x2a, 0x00]);
        assert_eq!(cpu.a, 0x01);
    }

    #[test]
    fn test_ror() {
        let mut cpu = CPU::new();
        test_cpu(&mut cpu, vec![0x38, 0x6a, 0x00]);
        assert_eq!(cpu.a, 0x80);
    }

    // todo add RTI test

    #[test]
    fn test_rts() {
        let mut cpu = CPU::new();
        test_cpu(&mut cpu, vec![0xa9, 0x80, 0x48, 0xa9, 0x06, 0x48, 0x60]);
        assert_eq!(cpu.pc, 0x8007);
    }

    #[test]
    fn test_sbc() {
        let mut cpu = CPU::new();
        test_cpu(&mut cpu, vec![0xe9, 0x01, 0xe9, 0x01]);
        assert_eq!(cpu.a, (!0x02u8).wrapping_add(1));
    }

    #[test]
    fn test_sec() {
        let mut cpu = CPU::new();
        test_cpu(&mut cpu, vec![0x38]);
        assert_ne!(cpu.p & FLG_C, 0);
    }

    #[test]
    fn test_sed() {
        let mut cpu = CPU::new();
        test_cpu(&mut cpu, vec![0xf8]);
        assert_ne!(cpu.p & FLG_D, 0);
    }

    #[test]
    fn test_sei() {
        let mut cpu = CPU::new();
        test_cpu(&mut cpu, vec![0x78]);
        assert_ne!(cpu.p & FLG_I, 0);
    }

    #[test]
    fn test_sta() {
        let mut cpu = CPU::new();
        test_cpu(&mut cpu, vec![0xa9, 0x55, 0x8d, 0x00, 0x00]);
        assert_eq!(cpu.read(0x0000), 0x55);
    }

    #[test]
    fn test_stx() {
        let mut cpu = CPU::new();
        test_cpu(&mut cpu, vec![0xa2, 0x55, 0x8e, 0x00, 0x00]);
        assert_eq!(cpu.read(0x0000), 0x55);
    }

    #[test]
    fn test_sty() {
        let mut cpu = CPU::new();
        test_cpu(&mut cpu, vec![0xa0, 0x55, 0x8c, 0x00, 0x00]);
        assert_eq!(cpu.read(0x0000), 0x55);
    }

    #[test]
    fn test_tax() {
        let mut cpu = CPU::new();
        test_cpu(&mut cpu, vec![0xa9, 0x55, 0xaa]);
        assert_eq!(cpu.x, 0x55);
    }

    #[test]
    fn test_tay() {
        let mut cpu = CPU::new();
        test_cpu(&mut cpu, vec![0xa9, 0x55, 0xa8]);
        assert_eq!(cpu.y, 0x55);
    }

    #[test]
    fn test_tsx() {
        let mut cpu = CPU::new();
        test_cpu(&mut cpu, vec![0xba]);
        assert_eq!(cpu.x, 0xfd);
    }

    #[test]
    fn test_txa() {
        let mut cpu = CPU::new();
        test_cpu(&mut cpu, vec![0xa2, 0x55, 0x8a]);
        assert_eq!(cpu.a, 0x55);
    }

    #[test]
    fn test_txs() {
        let mut cpu = CPU::new();
        test_cpu(&mut cpu, vec![0xa2, 0x55, 0x9a]);
        assert_eq!(cpu.s, 0x55);
    }

    #[test]
    fn test_tya() {
        let mut cpu = CPU::new();
        test_cpu(&mut cpu, vec![0xa0, 0x55, 0x98]);
        assert_eq!(cpu.a, 0x55);
    }
}