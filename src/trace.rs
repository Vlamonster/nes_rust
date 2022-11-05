use crate::cpu::{AddressingMode, Mem, CPU};
use crate::opcodes;
use std::collections::HashMap;

pub fn trace(cpu: &mut CPU) -> String {
    let opcodes: &HashMap<u8, &'static opcodes::OpCode> = &(*opcodes::OPCODES_MAP);

    let code = cpu.read(cpu.pc);
    let opcode = opcodes
        .get(&code)
        .expect(&*format!("OpCode {:#02x} was not found", code));

    let begin = cpu.pc;
    let mut hex_dump = vec![];
    hex_dump.push(code);

    let (adr, val) = match opcode.mode {
        AddressingMode::Immediate | AddressingMode::Implied => (0, 0),
        _ => {
            let adr = cpu.get_effective_address(&opcode.mode, begin + 1);
            (adr, cpu.read(adr))
        }
    };

    let tmp = match opcode.len {
        1 => match opcode.code {
            // Accumulator is operand
            0x0a | 0x4a | 0x2a | 0x6a => "A ".to_string(),
            _ => String::from(""),
        },
        2 => {
            let operand = cpu.read(begin + 1);
            hex_dump.push(operand);

            match opcode.mode {
                AddressingMode::Immediate => format!("#${:02x}", operand),
                AddressingMode::ZeroPage => format!("${:02x} = {:02x}", adr, val),
                AddressingMode::ZeroPageX => {
                    format!("${:02x},X @ {:02x} = {:02x}", operand, adr, val)
                }
                AddressingMode::ZeroPageY => {
                    format!("${:02x},Y @ {:02x} = {:02x}", operand, adr, val)
                }
                AddressingMode::IndirectX => format!(
                    "(${:02x},X) @ {:02x} = {:04x} = {:02x}",
                    operand,
                    operand.wrapping_add(cpu.x),
                    adr,
                    val
                ),
                AddressingMode::IndirectY => format!(
                    "(${:02x}),Y = {:04x} @ {:04x} = {:02x}",
                    operand,
                    adr.wrapping_sub(cpu.y as u16),
                    adr,
                    val
                ),
                AddressingMode::Implied =>
                // Operand is a relative offset from next instruction
                {
                    format!(
                        "${:04x}",
                        (begin as i16 + 2).wrapping_add(operand as i8 as i16) as u16
                    )
                }
                _ => panic!(
                    "Unexpected addressing mode {:?} of length 2 for opcode {:02x}",
                    opcode.mode, opcode.code
                ),
            }
        }
        3 => {
            let operand_lo = cpu.read(begin + 1);
            let operand_hi = cpu.read(begin + 2);

            hex_dump.push(operand_lo);
            hex_dump.push(operand_hi);

            let operand = cpu.read_address(begin + 1);

            match opcode.mode {
                AddressingMode::Indirect => {
                    format!("(${:04x}) = {:04x}", operand, adr)
                }
                AddressingMode::Absolute => {
                    // check whether JMP or JSR, or memory location
                    match opcode.code {
                        0x20 | 0x4c => format!("${:04x}", operand),
                        _ => format!("${:04x} = {:02x}", adr, val),
                    }
                }
                AddressingMode::AbsoluteX => {
                    format!("${:04x},X @ {:04x} = {:02x}", operand, adr, val)
                }
                AddressingMode::AbsoluteY => {
                    format!("${:04x},Y @ {:04x} = {:02x}", operand, adr, val)
                }
                _ => panic!(
                    "Unexpected addressing mode {:?} of length 3 for opcode {:02x}",
                    opcode.mode, opcode.code
                ),
            }
        }
        _ => String::from(""),
    };

    let hex_str = hex_dump
        .iter()
        .map(|x| format!("{:02x}", x))
        .collect::<Vec<String>>()
        .join(" ");
    let asm_str = format!(
        "{:04x}  {:8} {: >4} {}",
        begin, hex_str, opcode.mnemonic, tmp
    )
    .trim()
    .to_string();
    format!(
        "{:47} A:{:02x} X:{:02x} Y:{:02x} P:{:02x} SP:{:02x}",
        asm_str, cpu.a, cpu.x, cpu.y, cpu.p, cpu.s,
    )
    .to_ascii_uppercase()
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::bus::Bus;
    use crate::cartridge::test::test_rom;

    /// Takes a vector of program memory and test it with trace starting from 0x8000.
    fn test_cpu_trace(result: &mut Vec<String>, program: Vec<u8>) -> CPU {
        let program_size = program.len();
        let mut padded_program = program;
        padded_program.extend(vec![0; 2 * 0x4000 - program_size - 4]);
        padded_program.extend(vec![0x00, 0x80, 0x00, 0x00]);

        let bus = Bus::new(test_rom(padded_program), |_, _| {});
        let mut cpu = CPU::new(bus);
        cpu.reset();
        cpu.run_with_callback(
            |cpu| {
                result.push(trace(cpu));
            },
            true,
            program_size as u64,
        );

        cpu
    }

    #[test]
    fn test_format_trace() {
        let mut result: Vec<String> = vec![];
        test_cpu_trace(&mut result, vec![0xa2, 0x01, 0xca, 0x88]);

        assert_eq!(
            "8000  A2 01     LDX #$01                        A:00 X:00 Y:00 P:24 SP:FD",
            result[0]
        );
        assert_eq!(
            "8002  CA        DEX                             A:00 X:01 Y:00 P:24 SP:FD",
            result[1]
        );
        assert_eq!(
            "8003  88        DEY                             A:00 X:00 Y:00 P:26 SP:FD",
            result[2]
        );
    }
}
