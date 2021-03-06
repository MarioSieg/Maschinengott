use iced_x86::{Decoder, DecoderOptions, Formatter, GasFormatter, Instruction, IntelFormatter};
use rayon::prelude::*;
use std::collections::HashMap;

#[repr(u32)]
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Bitness {
    X64 = 64,
}

pub struct DisassemblerResult {
    pub assembly: Vec<String>,
    pub most_used_instructions: Vec<(String, usize)>,
    pub isa_extensions_used: Vec<String>,
}

pub fn disassemble(
    bytes: &[u8],
    bitness: Bitness,
    rip: u64,
    use_binary: bool,
    use_intel: bool,
) -> DisassemblerResult {
    let instructions: Vec<Instruction> = {
        let mut decoder = Decoder::with_ip(bitness as u32, bytes, rip, DecoderOptions::NONE);
        decoder.iter().collect()
    };

    let assembly = extract_assembly(&instructions, bytes, rip, use_binary, use_intel);
    let most_used_instructions = extract_most_used_instructions(&instructions);
    let isa_extensions_used = extract_isa_extensions(&instructions);

    DisassemblerResult {
        assembly,
        most_used_instructions,
        isa_extensions_used,
    }
}

fn extract_assembly(
    instructions: &[Instruction],
    bytes: &[u8],
    rip: u64,
    use_binary: bool,
    use_intel: bool,
) -> Vec<String> {
    let width = if use_binary { 64 + 16 } else { 32 };
    instructions
        .par_iter()
        .map(|&instruction| {
            let mut out = String::new();
            if use_intel {
                let mut formatter = IntelFormatter::new();
                let options = formatter.options_mut();
                options.set_uppercase_mnemonics(false);
                options.set_first_operand_char_index(8);
                formatter.format(&instruction, &mut out);
            } else {
                let mut formatter = GasFormatter::new();
                let options = formatter.options_mut();
                options.set_uppercase_mnemonics(false);
                options.set_gas_show_mnemonic_size_suffix(true);
                options.set_first_operand_char_index(8);
                formatter.format(&instruction, &mut out);
            };

            let mut line = if use_binary {
                format!("{:016b} ", instruction.ip())
            } else {
                format!("{:016X} ", instruction.ip())
            };

            let mut machine_code = String::new();
            let start_index = (instruction.ip() - rip) as usize;
            let instr_bytes = &bytes[start_index..start_index + instruction.len()];
            for b in instr_bytes.iter() {
                if use_binary {
                    machine_code = format!("{}{:08b} ", machine_code, b);
                } else {
                    machine_code = format!("{}{:02X} ", machine_code, b);
                }
            }

            line = format!(
                "{} | {:0width$} | {:<32} | {}\n",
                line,
                machine_code,
                instruction.op_code().op_code_string(),
                out,
                width = width
            );
            line
        })
        .collect::<Vec<String>>()
}

fn extract_most_used_instructions(instructions: &[Instruction]) -> Vec<(String, usize)> {
    let mut most_used = HashMap::<String, usize>::new();
    for instruction in instructions {
        let mnemonic = format!("{:?}", instruction.mnemonic()).to_lowercase();
        if let Some(x) = most_used.get_mut(&mnemonic) {
            *x += 1;
        } else {
            most_used.insert(mnemonic, 1);
        }
    }
    let mut most_used = most_used
        .into_iter()
        .map(|(k, v)| (k, v))
        .collect::<Vec<(String, usize)>>();
    most_used.sort_by(|(_, v1), (_, v2)| v2.cmp(v1));
    most_used
}

fn extract_isa_extensions(instructions: &[Instruction]) -> Vec<String> {
    let mut result = Vec::new();
    for instruction in instructions {
        for feature in instruction.cpuid_features() {
            let str = format!("{:?}", feature);
            if !result.contains(&str) {
                result.push(str);
            }
        }
    }
    result
}
