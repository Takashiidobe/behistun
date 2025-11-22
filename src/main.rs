mod cpu;
mod decoder;
mod loader;
mod memory;
mod syscall;

use std::{env, fs, path::PathBuf};

use goblin::Object;

use crate::{
    cpu::{Cpu, ElfInfo},
    loader::load_memory_image,
};
use anyhow::bail;

fn main() {
    if let Err(err) = run() {
        eprintln!("Error: {err}");
        std::process::exit(1);
    }
}

fn run() -> anyhow::Result<()> {
    let (binary_path, program_args) = parse_args()?;
    let data = fs::read(&binary_path)?;
    let elf = match Object::parse(&data)? {
        Object::Elf(elf) => elf,
        other => {
            bail!("Unsupported object format: {other:?}");
        }
    };

    let memory = load_memory_image(&elf, &data)?;

    // Find where program headers are loaded in memory
    // They're typically at the start of the first PT_LOAD segment + e_phoff
    let first_load_vaddr = elf
        .program_headers
        .iter()
        .find(|ph| ph.p_type == goblin::elf::program_header::PT_LOAD)
        .map(|ph| ph.p_vaddr)
        .unwrap_or(0x80000000);

    // Program headers are at file offset e_phoff, which maps to vaddr + (e_phoff - p_offset)
    // For typical ELFs, program headers are at vaddr + e_phoff when first segment starts at offset 0
    let phdr_addr = first_load_vaddr + elf.header.e_phoff;

    let elf_info = ElfInfo {
        entry_point: elf.entry as u32,
        phdr_addr: phdr_addr as u32,
        phent_size: elf.header.e_phentsize as u32,
        phnum: elf.header.e_phnum as u32,
        tls_vaddr: elf
            .program_headers
            .iter()
            .find(|ph| ph.p_type == goblin::elf::program_header::PT_TLS)
            .map(|ph| ph.p_vaddr as u32),
        tls_memsz: elf
            .program_headers
            .iter()
            .find(|ph| ph.p_type == goblin::elf::program_header::PT_TLS)
            .map(|ph| ph.p_memsz as u32)
            .unwrap_or(0),
    };

    let mut cpu = Cpu::new(memory.clone(), &elf_info, &program_args)?;

    // Use JIT mode - decode instructions on-the-fly as they're executed
    cpu.run(vec![])?;
    Ok(())
}

fn parse_args() -> anyhow::Result<(PathBuf, Vec<String>)> {
    let mut args: Vec<String> = env::args().skip(1).collect();
    if args.is_empty() {
        bail!("expected path to an ELF binary (usage: m68k-interp <binary> [args...])");
    }
    let binary_path = PathBuf::from(args.remove(0));
    // Canonicalize the path to get an absolute path (for /proc/self/exe)
    let canonical_path = binary_path.canonicalize()
        .unwrap_or_else(|_| binary_path.clone());
    // args now contains the arguments to pass to the emulated program
    // Prepend the canonical binary path as argv[0]
    let mut program_args = vec![canonical_path.to_string_lossy().into_owned()];
    program_args.extend(args);
    Ok((binary_path, program_args))
}
