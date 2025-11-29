use anyhow::{Result, anyhow, bail};
use goblin::{Object, elf::program_header};
use std::fs;

use crate::cpu::{ElfInfo, M68K_TLS_TCB_SIZE, align_up};
use crate::Cpu;

impl Cpu {
    /// execve(filename, argv, envp)
    /// Replace the current process with a new program
    pub(crate) fn sys_execve(&mut self) -> Result<i64> {
        let filename_addr = self.data_regs[1] as usize;
        let argv_addr = self.data_regs[2] as usize;
        let envp_addr = self.data_regs[3] as usize;

        // Read filename from guest memory
        let filename_cstr = self
            .guest_cstring(filename_addr)
            .map_err(|e| anyhow!("failed to read filename at {:#x}: {}", filename_addr, e))?;
        let filename = filename_cstr
            .to_str()
            .map_err(|e| anyhow!("invalid UTF-8 in filename: {}", e))?;

        // Read argv array (NULL-terminated array of string pointers)
        let argv = if argv_addr == 0 {
            vec![filename.to_string()] // Default to just the program name
        } else {
            self.read_string_array(argv_addr)?
        };

        // Read envp array (we'll ignore it for now since most tests don't use it)
        let _envp = if envp_addr == 0 {
            Vec::new()
        } else {
            self.read_string_array(envp_addr)?
        };

        // Load the new ELF binary
        let data = fs::read(filename).map_err(|e| anyhow!("failed to read {}: {}", filename, e))?;

        let elf = match Object::parse(&data)? {
            Object::Elf(elf) => elf,
            other => {
                bail!("execve: unsupported object format: {:?}", other);
            }
        };

        // Load the new memory image
        let new_memory = crate::loader::load_memory_image(&elf, &data)?;

        // Find where program headers are loaded in memory
        let first_load_vaddr = elf
            .program_headers
            .iter()
            .find(|ph| ph.p_type == program_header::PT_LOAD)
            .map(|ph| ph.p_vaddr)
            .unwrap_or(0x80000000);

        let phdr_addr = first_load_vaddr + elf.header.e_phoff;

        let elf_info = ElfInfo {
            entry_point: elf.entry as u32,
            phdr_addr: phdr_addr as u32,
            phent_size: elf.header.e_phentsize as u32,
            phnum: elf.header.e_phnum as u32,
            tls_vaddr: elf
                .program_headers
                .iter()
                .find(|ph| ph.p_type == program_header::PT_TLS)
                .map(|ph| ph.p_vaddr as u32),
            tls_memsz: elf
                .program_headers
                .iter()
                .find(|ph| ph.p_type == program_header::PT_TLS)
                .map(|ph| ph.p_memsz as u32)
                .unwrap_or(0),
        };

        // Replace memory and reset CPU state
        self.memory = new_memory;

        // Reset registers
        self.data_regs = [0; 8];
        self.addr_regs = [0; 8];
        self.sr = 0;
        self.pc = elf_info.entry_point as usize;
        self.halted = false;

        // Update exe_path to the new binary being executed
        self.exe_path = argv
            .first()
            .map(|s| s.to_string())
            .unwrap_or_else(|| filename.to_string());

        // Recalculate stack_base, brk, etc.
        let (stack_base, stack_index) = self
            .memory
            .segments()
            .iter()
            .enumerate()
            .map(|(idx, seg)| (seg.vaddr, idx))
            .max_by_key(|(vaddr, _)| *vaddr)
            .ok_or_else(|| anyhow!("no segments in new memory image"))?;

        self.stack_base = stack_base;

        // Find the highest writable program segment (exclude the stack)
        let mut heap_segment_base = None;
        let mut heap_segment_end = 0usize;
        for (idx, seg) in self.memory.segments().iter().enumerate() {
            if idx == stack_index {
                continue;
            }
            if (seg.flags & program_header::PF_W) != 0 {
                let end = seg.vaddr + seg.len();
                if end > heap_segment_end {
                    heap_segment_end = end;
                    heap_segment_base = Some(seg.vaddr);
                }
            }
        }

        let heap_segment_base =
            heap_segment_base.ok_or_else(|| anyhow!("no writable segment for heap"))?;
        self.heap_segment_base = heap_segment_base;

        // Set up TLS
        let tls_base = elf_info
            .tls_vaddr
            .map(|v| v as usize + M68K_TLS_TCB_SIZE)
            .unwrap_or(0);
        self.tls_base = tls_base as u32;
        self.tls_initialized = false;
        self.tls_memsz = elf_info.tls_memsz as usize;

        // Set up brk
        let mut brk_base = align_up(heap_segment_end, 4096);
        if tls_base != 0 {
            brk_base = brk_base.max(align_up(tls_base, 4096));
        }
        if brk_base > heap_segment_end {
            self.memory
                .resize_segment(heap_segment_base, brk_base - heap_segment_base)?;
        }
        self.brk = brk_base;
        self.brk_base = brk_base;

        // Initialize TLS if needed
        if tls_base != 0 {
            self.ensure_tls_range(tls_base)?;
        }

        // Set up the initial stack with new argc/argv/envp
        self.setup_initial_stack(&argv, &elf_info)?;

        // execve doesn't return on success - it replaces the current process
        // We need to restart execution with the new memory image
        // The decoder in run_jit() has a clone of the old memory, so we need to
        // restart the run loop to create a new decoder with the new memory
        self.run_jit()?;

        // If run_jit returns (program exited), we should exit this process too
        std::process::exit(0);
    }
}
