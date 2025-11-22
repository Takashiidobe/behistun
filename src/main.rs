mod decoder;
mod loader;
mod memory;

use std::{env, error::Error, fs, path::PathBuf};

use goblin::Object;

use crate::{decoder::Decoder, loader::load_memory_image};

fn main() {
    if let Err(err) = run() {
        eprintln!("Error: {err}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), Box<dyn Error>> {
    let binary_path = parse_args()?;
    let data = fs::read(&binary_path)?;
    let elf = match Object::parse(&data)? {
        Object::Elf(elf) => elf,
        other => {
            return Err(format!("Unsupported object format: {other:?}").into());
        }
    };

    let memory = load_memory_image(&elf, &data)?;

    println!();
    println!("Finding unknown instructions in executable sections...");

    let decoder = Decoder::new(memory);

    for section in &elf.section_headers {
        if (section.sh_flags & goblin::elf::section_header::SHF_EXECINSTR as u64) != 0 {
            let start = section.sh_addr as usize;
            let end = start + section.sh_size as usize;

            let instructions = decoder.decode_instructions(start, end)?;
            for instruction in instructions {
                println!("{}", instruction);
            }
        }
    }

    println!();
    println!("Decoder run complete.");
    Ok(())
}

fn parse_args() -> Result<PathBuf, Box<dyn Error>> {
    let mut args = env::args().skip(1);
    match args.next() {
        Some(path) => Ok(PathBuf::from(path)),
        None => Err("expected path to an ELF binary (usage: m68k-interp <binary>)".into()),
    }
}
