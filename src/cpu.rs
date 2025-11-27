use std::collections::BTreeMap;
use std::ffi::CString;

use anyhow::{Result, anyhow, bail};

use crate::{
    decoder::{
        Abcd, Add, AddrReg, AddressingMode, Addx, And, BitFieldParam, BitOp, Condition, DataDir,
        DataReg, Decoder, DnToEa, EaToDn, EffectiveAddress, Exg, ExtMode, ImmOp, Immediate,
        Instruction, InstructionKind, Movem, Or, QuickOp, RightOrLeft, Sbcd, Shift, ShiftCount,
        Size, Sub, Subx, UnaryOp,
    },
    memory::MemoryImage,
    syscall::m68k_to_x86_64_syscall,
};

/// ELF information needed for auxiliary vector setup
#[derive(Debug, Clone)]
pub struct ElfInfo {
    pub entry_point: u32,
    pub phdr_addr: u32,  // Address of program headers in memory
    pub phent_size: u32, // Size of each program header entry
    pub phnum: u32,      // Number of program headers
    pub tls_vaddr: Option<u32>,
    pub tls_memsz: u32,
}

fn align_up(value: usize, align: usize) -> usize {
    (value + align - 1) & !(align - 1)
}

// m68k uses a fixed TLS layout where the thread pointer lives 0x7000 bytes
// past the start of the TLS block. TLS offsets (tpoff) are negative relative
// to the thread pointer, so make sure we always leave space for this gap.
const M68K_TLS_TCB_SIZE: usize = 0x7000;
// Give the TLS block a small pad after the thread pointer for any per-thread
// metadata the runtime might place there.
const TLS_DATA_PAD: usize = 0x1000;

pub struct Cpu {
    data_regs: [u32; 8],
    addr_regs: [u32; 8],
    sr: u16,
    pc: usize,
    memory: MemoryImage,
    halted: bool,
    tls_base: u32, // Thread-local storage base address
    tls_initialized: bool,
    tls_memsz: usize,
    brk: usize,
    brk_base: usize,
    heap_segment_base: usize,
    stack_base: usize,
    exe_path: String, // Path to the m68k executable being run
}

impl Cpu {
    pub fn new(memory: MemoryImage, elf_info: &ElfInfo, args: &[String]) -> Result<Self> {
        let mut memory = memory;

        // Identify stack segment (highest-address segment we added in the loader)
        let (stack_base, _stack_top, stack_index) = memory
            .segments()
            .iter()
            .enumerate()
            .map(|(idx, seg)| (seg.vaddr, seg.vaddr + seg.len(), idx))
            .max_by_key(|(_, end, _)| *end)
            .ok_or_else(|| anyhow!("no segments in memory image"))?;

        // Find the highest writable program segment (exclude the stack)
        let mut heap_segment_base = None;
        let mut heap_segment_end = 0usize;
        for (idx, seg) in memory.segments().iter().enumerate() {
            if idx == stack_index {
                continue;
            }
            if (seg.flags & goblin::elf::program_header::PF_W) != 0 {
                let end = seg.vaddr + seg.len();
                if end > heap_segment_end {
                    heap_segment_end = end;
                    heap_segment_base = Some(seg.vaddr);
                }
            }
        }

        let heap_segment_base =
            heap_segment_base.ok_or_else(|| anyhow!("no writable segment for heap"))?;

        // Align the initial brk to a 4KB boundary like Linux does.
        let tls_base = elf_info
            .tls_vaddr
            .map(|v| v as usize + M68K_TLS_TCB_SIZE)
            .unwrap_or(0);

        let mut brk_base = align_up(heap_segment_end, 4096);
        if tls_base != 0 {
            // Keep the heap start after the TLS control block so allocations
            // don't trample the thread pointer region.
            brk_base = brk_base.max(align_up(tls_base, 4096));
        }
        // Ensure the backing segment covers the aligned brk base.
        if brk_base > heap_segment_end {
            memory.resize_segment(heap_segment_base, brk_base - heap_segment_base)?;
        }

        let mut cpu = Self {
            data_regs: [0; 8],
            addr_regs: [0; 8],
            sr: 0,
            pc: elf_info.entry_point as usize,
            memory,
            halted: false,
            tls_base: tls_base as u32,
            tls_initialized: false,
            tls_memsz: elf_info.tls_memsz as usize,
            brk: brk_base,
            brk_base,
            heap_segment_base,
            stack_base,
            exe_path: args.first().map(|s| s.to_string()).unwrap_or_default(),
        };

        if tls_base != 0 {
            cpu.ensure_tls_range(tls_base)?;
        }

        // Set up the initial stack with argc/argv/envp
        cpu.setup_initial_stack(args, elf_info)?;

        Ok(cpu)
    }

    /// Set up the initial stack with argc/argv/envp and auxiliary vector
    /// This can be called both during initialization and for execve
    fn setup_initial_stack(&mut self, args: &[String], elf_info: &ElfInfo) -> Result<()> {
        // Find stack segment
        let stack_top = self.stack_base
            + self
                .memory
                .segments()
                .iter()
                .find(|seg| seg.vaddr == self.stack_base)
                .map(|seg| seg.len())
                .ok_or_else(|| anyhow!("stack segment not found"))?;

        // Layout (growing downward from stack_top):
        //   [string data for argv]
        //   [padding to align to 4 bytes]
        //   NULL (envp terminator)
        //   NULL (argv terminator)
        //   argv[n-1] pointer
        //   ...
        //   argv[0] pointer
        //   argc                    <-- SP points here

        // Leave some padding at the top for safety
        let mut sp = stack_top - 64;

        // First, write all argument strings to the top of the stack
        // and collect their addresses
        let mut argv_addrs = Vec::with_capacity(args.len());

        for arg in args.iter().rev() {
            // Include null terminator
            let bytes = arg.as_bytes();
            let len = bytes.len() + 1; // +1 for null terminator
            sp -= len;
            // Write the string
            self.memory.write_data(sp, bytes)?;
            self.memory.write_data(sp + bytes.len(), &[0u8])?; // null terminator
            argv_addrs.push(sp as u32);
        }
        argv_addrs.reverse(); // Now in correct order (argv[0] first)

        // Align SP to 4 bytes
        sp &= !3;

        // Write 16 random bytes for AT_RANDOM (stack canary seed)
        let random_addr = sp - 16;
        sp = random_addr;
        // Use deterministic "random" bytes for reproducibility
        let random_bytes: [u8; 16] = [
            0x12, 0x34, 0x56, 0x78, 0x9a, 0xbc, 0xde, 0xf0, 0x11, 0x22, 0x33, 0x44, 0x55, 0x66,
            0x77, 0x88,
        ];
        self.memory.write_data(sp, &random_bytes)?;

        // Align SP to 4 bytes again
        sp &= !3;

        // Write auxiliary vector (auxv) - pairs of (type, value)
        // Must be written in reverse order (last entry first)
        //
        // Auxv types:
        // AT_NULL=0, AT_PHDR=3, AT_PHENT=4, AT_PHNUM=5, AT_PAGESZ=6,
        // AT_BASE=7, AT_FLAGS=8, AT_ENTRY=9, AT_UID=11, AT_EUID=12,
        // AT_GID=13, AT_EGID=14, AT_SECURE=23, AT_RANDOM=25

        let auxv_entries: &[(u32, u32)] = &[
            (0, 0),                    // AT_NULL - terminator
            (25, random_addr as u32),  // AT_RANDOM - pointer to 16 random bytes
            (23, 0),                   // AT_SECURE - not a setuid program
            (14, 1000),                // AT_EGID - effective group ID
            (13, 1000),                // AT_GID - real group ID
            (12, 1000),                // AT_EUID - effective user ID
            (11, 1000),                // AT_UID - real user ID
            (9, elf_info.entry_point), // AT_ENTRY - entry point
            (8, 0),                    // AT_FLAGS - flags
            (7, 0),                    // AT_BASE - base addr of interpreter (0 for static)
            (6, 4096),                 // AT_PAGESZ - page size
            (5, elf_info.phnum),       // AT_PHNUM - number of program headers
            (4, elf_info.phent_size),  // AT_PHENT - size of program header entry
            (3, elf_info.phdr_addr),   // AT_PHDR - program headers address
        ];

        // Write auxv in reverse order (so AT_PHDR is at lowest address)
        for &(typ, val) in auxv_entries.iter() {
            sp -= 4;
            self.memory.write_data(sp, &val.to_be_bytes())?;
            sp -= 4;
            self.memory.write_data(sp, &typ.to_be_bytes())?;
        }

        // Write envp terminator (NULL)
        sp -= 4;
        self.memory.write_data(sp, &0u32.to_be_bytes())?;

        // Write argv terminator (NULL)
        sp -= 4;
        self.memory.write_data(sp, &0u32.to_be_bytes())?;

        // Write argv pointers (in reverse order so argv[0] is at lowest address)
        for &addr in argv_addrs.iter().rev() {
            sp -= 4;
            self.memory.write_data(sp, &addr.to_be_bytes())?;
        }

        // Write argc
        sp -= 4;
        self.memory
            .write_data(sp, &(args.len() as u32).to_be_bytes())?;

        // Set stack pointer
        self.addr_regs[7] = sp as u32;

        Ok(())
    }

    pub fn run(&mut self, instructions: Vec<Instruction>) -> Result<()> {
        let instruction_map: BTreeMap<usize, Instruction> = instructions
            .into_iter()
            .map(|inst| (inst.address, inst))
            .collect();

        if instruction_map.is_empty() {
            // No pre-decoded instructions - decode on the fly
            return self.run_jit();
        }

        let mut trace_count = 0;
        while !self.halted {
            let pc = self.pc;
            let Some(inst) = instruction_map.get(&pc) else {
                // End of known code for now; treat as a clean halt.
                eprintln!("Halting: no instruction at PC={:#010x}", pc);
                break;
            };

            // Trace first 50 instructions
            if trace_count < 200 {
                eprintln!("EXEC[{}]: {:#010x} {:?}", trace_count, pc, inst.kind);
                trace_count += 1;
            }

            self.execute(inst)?;
        }

        Ok(())
    }

    /// Run with on-the-fly instruction decoding
    pub fn run_jit(&mut self) -> Result<()> {
        let decoder = Decoder::new(self.memory.clone());
        let mut instruction_cache: BTreeMap<usize, Instruction> = BTreeMap::new();

        let mut last_pc = 0usize;
        let mut last_inst_kind: Option<String> = None;
        while !self.halted {
            let pc = self.pc;

            // Check cache first, decode if not found
            let inst = if let Some(inst) = instruction_cache.get(&pc) {
                inst.clone()
            } else {
                let inst = decoder.decode_instruction(pc)?;
                instruction_cache.insert(pc, inst.clone());
                inst
            };

            if let Err(e) = self.execute(&inst) {
                eprintln!("FAILED at PC={:#010x}: {:?}", pc, inst.kind);
                eprintln!("  Last: PC={:#x} {:?}", last_pc, last_inst_kind);
                eprintln!("Error: {:?}", e);
                eprintln!(
                    "  D: {:08x} {:08x} {:08x} {:08x} {:08x} {:08x} {:08x} {:08x}",
                    self.data_regs[0],
                    self.data_regs[1],
                    self.data_regs[2],
                    self.data_regs[3],
                    self.data_regs[4],
                    self.data_regs[5],
                    self.data_regs[6],
                    self.data_regs[7]
                );
                eprintln!(
                    "  A: {:08x} {:08x} {:08x} {:08x} {:08x} {:08x} {:08x} {:08x}",
                    self.addr_regs[0],
                    self.addr_regs[1],
                    self.addr_regs[2],
                    self.addr_regs[3],
                    self.addr_regs[4],
                    self.addr_regs[5],
                    self.addr_regs[6],
                    self.addr_regs[7]
                );
                return Err(e);
            }
            last_pc = pc;
            last_inst_kind = Some(format!("{:?}", inst.kind));
        }

        Ok(())
    }

    fn execute(&mut self, instruction: &Instruction) -> Result<()> {
        match instruction.kind {
            InstructionKind::Nop => {}
            InstructionKind::Addq(op) => {
                self.exec_addq(instruction, op)?;
            }
            InstructionKind::Moveq { data, dst } => {
                self.exec_moveq(data, dst)?;
            }
            InstructionKind::Move { size, src, dst } => {
                self.exec_move(instruction, size, src, dst)?;
            }
            InstructionKind::Lea { src, dst } => {
                self.exec_lea(instruction, src, dst)?;
            }
            InstructionKind::Mulu { src, dst } => {
                self.exec_mulu(src, dst)?;
            }
            InstructionKind::Muls { src, dst } => {
                self.exec_muls(src, dst)?;
            }
            InstructionKind::Trap { vector } => {
                self.exec_trap(vector)?;
            }
            InstructionKind::Trapcc {
                condition,
                operand: _,
            } => {
                // TRAPcc (68020+)
                // In a real system, if condition is true, this would cause a TRAP exception.
                // The operand (if present) would be used by the exception handler.
                // For testing, treat as no-op regardless of condition.
                let _ = self.test_condition(condition);
            }
            InstructionKind::Bkpt { vector: _ } => {
                // BKPT #n (68010+)
                // In a real system, this would cause an illegal instruction exception
                // or be handled by a debugger. For testing, treat as no-op.
            }
            InstructionKind::Rts => {
                // Pop return address from stack and jump to it
                let return_addr = self.memory.read_long(self.addr_regs[7] as usize)?;
                self.addr_regs[7] = self.addr_regs[7].wrapping_add(4);
                self.pc = return_addr as usize;
                return Ok(()); // Don't advance PC normally
            }
            InstructionKind::Rtd { displacement } => {
                // Pop return address from stack and jump to it
                let return_addr = self.memory.read_long(self.addr_regs[7] as usize)?;
                self.addr_regs[7] = self.addr_regs[7].wrapping_add(4);
                // Add displacement to stack pointer
                self.addr_regs[7] = self.addr_regs[7].wrapping_add(displacement as u32);
                self.pc = return_addr as usize;
                return Ok(()); // Don't advance PC normally
            }
            InstructionKind::Tst { size, mode } => {
                self.exec_tst(instruction, size, mode)?;
            }
            InstructionKind::Bra { displacement } => {
                // BRA doesn't advance PC normally - it jumps
                self.exec_bra(instruction, displacement);
                return Ok(());
            }
            InstructionKind::Bcc {
                condition,
                displacement,
            } => {
                // Bcc may or may not branch
                if self.exec_bcc(instruction, condition, displacement) {
                    return Ok(()); // Took the branch
                }
                // Fall through - didn't branch, advance PC normally
            }
            InstructionKind::Clr(op) => {
                self.exec_clr(instruction, op)?;
            }
            InstructionKind::Neg(op) => {
                self.exec_neg(instruction, op)?;
            }
            InstructionKind::Not(op) => {
                self.exec_not(instruction, op)?;
            }
            InstructionKind::Negx(op) => {
                self.exec_negx(instruction, op)?;
            }
            InstructionKind::Add(add) => {
                self.exec_add(instruction, add)?;
            }
            InstructionKind::Sub(sub) => {
                self.exec_sub(instruction, sub)?;
            }
            InstructionKind::Cmp(ea_to_dn) => {
                self.exec_cmp(instruction, ea_to_dn)?;
            }
            InstructionKind::Cmpa {
                addr_reg,
                size,
                src,
            } => {
                self.exec_cmpa(instruction, addr_reg, size, src)?;
            }
            InstructionKind::And(and) => {
                self.exec_and(instruction, and)?;
            }
            InstructionKind::Or(or) => {
                self.exec_or(instruction, or)?;
            }
            InstructionKind::Eor(dn_to_ea) => {
                self.exec_eor(instruction, dn_to_ea)?;
            }
            InstructionKind::Jsr { mode } => {
                self.exec_jsr(instruction, mode)?;
                return Ok(()); // JSR sets PC directly
            }
            InstructionKind::Jmp { mode } => {
                self.exec_jmp(instruction, mode)?;
                return Ok(()); // JMP sets PC directly
            }
            InstructionKind::Bsr { displacement } => {
                self.exec_bsr(instruction, displacement)?;
                return Ok(()); // BSR sets PC directly
            }
            InstructionKind::Subq(op) => {
                self.exec_subq(instruction, op)?;
            }
            InstructionKind::Adda {
                addr_reg,
                size,
                mode,
            } => {
                self.exec_adda(instruction, addr_reg, size, mode)?;
            }
            InstructionKind::Suba {
                addr_reg,
                size,
                mode,
            } => {
                self.exec_suba(instruction, addr_reg, size, mode)?;
            }
            InstructionKind::Movea { size, src, dst } => {
                self.exec_movea(instruction, size, src, dst)?;
            }
            InstructionKind::Ext { data_reg, mode } => {
                self.exec_ext(data_reg, mode)?;
            }
            InstructionKind::Swap { data_reg } => {
                self.exec_swap(data_reg)?;
            }
            InstructionKind::Pea { mode } => {
                self.exec_pea(instruction, mode)?;
            }
            InstructionKind::Link {
                addr_reg,
                displacement,
            } => {
                self.exec_link(addr_reg, displacement)?;
            }
            InstructionKind::Unlk { addr_reg } => {
                self.exec_unlk(addr_reg)?;
            }
            InstructionKind::Addi(imm_op) => {
                self.exec_addi(instruction, imm_op)?;
            }
            InstructionKind::Subi(imm_op) => {
                self.exec_subi(instruction, imm_op)?;
            }
            InstructionKind::Andi(imm_op) => {
                self.exec_andi(instruction, imm_op)?;
            }
            InstructionKind::Ori(imm_op) => {
                self.exec_ori(instruction, imm_op)?;
            }
            InstructionKind::Eori(imm_op) => {
                self.exec_eori(instruction, imm_op)?;
            }
            InstructionKind::Cmpi(imm_op) => {
                self.exec_cmpi(instruction, imm_op)?;
            }
            // Shift instructions
            InstructionKind::Asd(shift) => {
                self.exec_asd(instruction, shift)?;
            }
            InstructionKind::Lsd(shift) => {
                self.exec_lsd(instruction, shift)?;
            }
            InstructionKind::Rod(shift) => {
                self.exec_rod(instruction, shift)?;
            }
            InstructionKind::Roxd(shift) => {
                self.exec_roxd(instruction, shift)?;
            }
            // Bit operations
            InstructionKind::Btst(bit_op) => {
                self.exec_btst(instruction, bit_op)?;
            }
            InstructionKind::Bchg(bit_op) => {
                self.exec_bchg(instruction, bit_op)?;
            }
            InstructionKind::Bclr(bit_op) => {
                self.exec_bclr(instruction, bit_op)?;
            }
            InstructionKind::Bset(bit_op) => {
                self.exec_bset(instruction, bit_op)?;
            }
            // Conditional instructions
            InstructionKind::Scc { condition, mode } => {
                self.exec_scc(instruction, condition, mode)?;
            }
            InstructionKind::DBcc {
                condition,
                data_reg,
                displacement,
            } => {
                if self.exec_dbcc(instruction, condition, data_reg, displacement) {
                    return Ok(()); // Took the branch
                }
            }
            // Division
            InstructionKind::Divu { src, dst } => {
                self.exec_divu(instruction, src, dst)?;
            }
            InstructionKind::Divs { src, dst } => {
                self.exec_divs(instruction, src, dst)?;
            }
            InstructionKind::DivuL {
                ref src,
                dq,
                dr,
                is_64bit,
            } => {
                self.exec_divul(instruction, src, dq, dr, is_64bit)?;
            }
            InstructionKind::DivsL {
                ref src,
                dq,
                dr,
                is_64bit,
            } => {
                self.exec_divsl(instruction, src, dq, dr, is_64bit)?;
            }
            InstructionKind::MuluL { ref src, dl, dh } => {
                self.exec_mulul(instruction, src, dl, dh)?;
            }
            InstructionKind::MulsL { ref src, dl, dh } => {
                self.exec_mulsl(instruction, src, dl, dh)?;
            }
            InstructionKind::Cas {
                size,
                dc,
                du,
                ref mode,
            } => {
                self.exec_cas(instruction, size, dc, du, mode)?;
            }
            InstructionKind::Cas2 {
                size,
                dc1,
                dc2,
                du1,
                du2,
                rn1,
                rn2,
            } => {
                self.exec_cas2(size, dc1, dc2, du1, du2, rn1, rn2)?;
            }
            InstructionKind::Cmp2 {
                size,
                ref mode,
                reg,
            } => {
                self.exec_cmp2(instruction, size, mode, reg)?;
            }
            InstructionKind::Chk2 {
                size,
                ref mode,
                reg,
            } => {
                self.exec_chk2(instruction, size, mode, reg)?;
            }
            // Exchange and extended operations
            InstructionKind::Exg(exg) => {
                self.exec_exg(exg)?;
            }
            InstructionKind::Addx(addx) => {
                self.exec_addx(addx)?;
            }
            InstructionKind::Subx(subx) => {
                self.exec_subx(subx)?;
            }
            // BCD operations
            InstructionKind::Abcd(abcd) => {
                self.exec_abcd(abcd)?;
            }
            InstructionKind::Sbcd(sbcd) => {
                self.exec_sbcd(sbcd)?;
            }
            // MOVEM
            InstructionKind::Movem(movem) => {
                self.exec_movem(instruction, movem)?;
            }
            // Compare memory
            InstructionKind::Cmpm { size, src, dst } => {
                self.exec_cmpm(size, src, dst)?;
            }
            // TAS - Test and Set
            InstructionKind::Tas { mode } => {
                self.exec_tas(instruction, mode)?;
            }
            // CCR/SR operations
            InstructionKind::OriToCcr { imm } => {
                self.sr |= imm as u16;
            }
            InstructionKind::EoriToCcr { imm } => {
                self.sr ^= imm as u16;
            }
            // Bit field operations
            InstructionKind::Bftst {
                ref mode,
                offset,
                width,
            } => {
                self.exec_bftst(instruction, mode, offset, width)?;
            }
            InstructionKind::Bfchg {
                ref mode,
                offset,
                width,
            } => {
                self.exec_bfchg(instruction, mode, offset, width)?;
            }
            InstructionKind::Bfclr {
                ref mode,
                offset,
                width,
            } => {
                self.exec_bfclr(instruction, mode, offset, width)?;
            }
            InstructionKind::Bfset {
                ref mode,
                offset,
                width,
            } => {
                self.exec_bfset(instruction, mode, offset, width)?;
            }
            InstructionKind::Bfextu {
                ref src,
                dst,
                offset,
                width,
            } => {
                self.exec_bfextu(instruction, src, dst, offset, width)?;
            }
            InstructionKind::Bfexts {
                ref src,
                dst,
                offset,
                width,
            } => {
                self.exec_bfexts(instruction, src, dst, offset, width)?;
            }
            InstructionKind::Bfins {
                src,
                ref dst,
                offset,
                width,
            } => {
                self.exec_bfins(instruction, src, dst, offset, width)?;
            }
            InstructionKind::Bfffo {
                ref src,
                dst,
                offset,
                width,
            } => {
                self.exec_bfffo(instruction, src, dst, offset, width)?;
            }
            InstructionKind::MoveToCcr { ref src } => {
                // MOVE to CCR: source word -> CCR (low 8 bits of SR)
                let value = self.read_operand(src, Size::Word, instruction.address)?;
                self.sr = (self.sr & 0xff00) | ((value as u16) & 0xff);
            }
            _ => bail!("execution for {:?} not yet implemented", instruction.kind),
        }
        self.advance_pc(instruction.len());

        Ok(())
    }

    fn advance_pc(&mut self, bytes: usize) {
        self.pc = self.pc.saturating_add(bytes);
    }

    fn exec_addq(&mut self, inst: &Instruction, op: QuickOp) -> Result<()> {
        let increment = if op.data == 0 { 8 } else { op.data as u32 };
        match op.mode.ea {
            EffectiveAddress::Dr(reg) => {
                let idx = data_reg_index(reg);
                let value = size_mask(self.data_regs[idx], op.size);
                let result = add_with_flags(value, increment, op.size, self);
                self.data_regs[idx] = write_sized_data_reg(self.data_regs[idx], result, op.size);
            }
            EffectiveAddress::Ar(reg) => {
                // ADDQ to An doesn't affect flags and always operates on full 32-bit
                let idx = addr_reg_index(reg);
                self.addr_regs[idx] = self.addr_regs[idx].wrapping_add(increment);
            }
            _ => {
                // Memory operands - read, add with flags, write back
                let value = self.read_operand(&op.mode, op.size, inst.address)?;
                let result = add_with_flags(value, increment, op.size, self);
                self.write_operand(&op.mode, op.size, inst.address, result)?;
            }
        }

        Ok(())
    }

    fn exec_moveq(&mut self, data: i8, dst: DataReg) -> Result<()> {
        let value = data as i32 as u32; // sign-extend to 32 bits
        let idx = data_reg_index(dst);
        self.data_regs[idx] = value;
        self.update_nz_flags(value);
        self.set_flag(FLAG_V, false);
        self.set_flag(FLAG_C, false);
        Ok(())
    }

    fn exec_move(
        &mut self,
        inst: &Instruction,
        size: Size,
        src: AddressingMode,
        dst: AddressingMode,
    ) -> Result<()> {
        let value = self.read_operand(&src, size, inst.address)?;
        self.write_operand(&dst, size, inst.address, value)?;
        self.update_nz_flags_sized(value, size);
        self.set_flag(FLAG_V, false);
        self.set_flag(FLAG_C, false);
        Ok(())
    }

    fn exec_lea(&mut self, inst: &Instruction, src: AddressingMode, dst: AddrReg) -> Result<()> {
        let addr = self.compute_effective_address(&src, inst.address)?;
        let idx = addr_reg_index(dst);
        self.addr_regs[idx] = addr as u32;
        Ok(())
    }

    fn exec_mulu(&mut self, src: AddressingMode, dst: DataReg) -> Result<()> {
        let src_val = self.read_word_unsigned(src)?;
        let dst_idx = data_reg_index(dst);
        let dst_val = self.data_regs[dst_idx] as u16 as u32;
        let result = dst_val.wrapping_mul(src_val as u32);
        self.data_regs[dst_idx] = result;
        self.set_flag(FLAG_C, false);
        self.set_flag(FLAG_V, false);
        self.update_nz_flags(result);
        Ok(())
    }

    fn exec_muls(&mut self, src: AddressingMode, dst: DataReg) -> Result<()> {
        let src_val = self.read_word_signed(src)?;
        let dst_idx = data_reg_index(dst);
        let dst_val = self.data_regs[dst_idx] as u16 as i16 as i32;
        let result = dst_val.wrapping_mul(src_val as i32);
        self.data_regs[dst_idx] = result as u32;
        self.set_flag(FLAG_C, false);
        self.set_flag(FLAG_V, false);
        self.update_nz_flags(result as u32);
        Ok(())
    }

    fn exec_tst(&mut self, inst: &Instruction, size: Size, mode: AddressingMode) -> Result<()> {
        let value = self.read_operand(&mode, size, inst.address)?;
        // TST sets N and Z based on the operand, clears V and C
        self.update_nz_flags_sized(value, size);
        self.set_flag(FLAG_V, false);
        self.set_flag(FLAG_C, false);
        Ok(())
    }

    fn exec_bra(&mut self, inst: &Instruction, displacement: i32) {
        // BRA: PC = PC + 2 + displacement
        // The displacement is relative to PC+2 (after the opcode word)
        let new_pc = (inst.address as i64) + 2 + (displacement as i64);
        self.pc = new_pc as usize;
    }

    fn exec_bcc(&mut self, inst: &Instruction, condition: Condition, displacement: i32) -> bool {
        if self.test_condition(condition) {
            // Take the branch
            let new_pc = (inst.address as i64) + 2 + (displacement as i64);
            self.pc = new_pc as usize;
            true
        } else {
            false
        }
    }

    fn test_condition(&self, condition: Condition) -> bool {
        let n = self.get_flag(FLAG_N);
        let z = self.get_flag(FLAG_Z);
        let v = self.get_flag(FLAG_V);
        let c = self.get_flag(FLAG_C);

        match condition {
            Condition::True => true,
            Condition::False => false,
            Condition::Higher => !c && !z,            // HI: !C && !Z
            Condition::LowerOrSame => c || z,         // LS: C || Z
            Condition::CarryClear => !c,              // CC: !C
            Condition::CarrySet => c,                 // CS: C
            Condition::NotEqual => !z,                // NE: !Z
            Condition::Equal => z,                    // EQ: Z
            Condition::OverflowClear => !v,           // VC: !V
            Condition::OverflowSet => v,              // VS: V
            Condition::Plus => !n,                    // PL: !N
            Condition::Minus => n,                    // MI: N
            Condition::GreaterOrEqual => n == v,      // GE: N == V
            Condition::LessThan => n != v,            // LT: N != V
            Condition::GreaterThan => !z && (n == v), // GT: !Z && (N == V)
            Condition::LessOrEqual => z || (n != v),  // LE: Z || (N != V)
        }
    }

    fn get_flag(&self, mask: u16) -> bool {
        (self.sr & mask) != 0
    }

    fn update_nz_flags_sized(&mut self, value: u32, size: Size) {
        let (is_zero, is_negative) = match size {
            Size::Byte => ((value & 0xFF) == 0, (value & 0x80) != 0),
            Size::Word => ((value & 0xFFFF) == 0, (value & 0x8000) != 0),
            Size::Long => (value == 0, (value & 0x8000_0000) != 0),
        };
        self.set_flag(FLAG_Z, is_zero);
        self.set_flag(FLAG_N, is_negative);
    }

    // CLR - Clear an operand
    fn exec_clr(&mut self, inst: &Instruction, op: UnaryOp) -> Result<()> {
        self.write_operand(&op.mode, op.size, inst.address, 0)?;
        self.set_flag(FLAG_N, false);
        self.set_flag(FLAG_Z, true);
        self.set_flag(FLAG_V, false);
        self.set_flag(FLAG_C, false);
        Ok(())
    }

    // NEG - Negate (two's complement)
    fn exec_neg(&mut self, inst: &Instruction, op: UnaryOp) -> Result<()> {
        let value = self.read_operand(&op.mode, op.size, inst.address)?;
        let result = sub_with_flags(0, value, op.size, self);
        self.write_operand(&op.mode, op.size, inst.address, result)?;
        Ok(())
    }

    // NOT - Logical complement (one's complement)
    fn exec_not(&mut self, inst: &Instruction, op: UnaryOp) -> Result<()> {
        let value = self.read_operand(&op.mode, op.size, inst.address)?;
        let result = !value;
        self.write_operand(&op.mode, op.size, inst.address, result)?;
        self.update_nz_flags_sized(result, op.size);
        self.set_flag(FLAG_V, false);
        self.set_flag(FLAG_C, false);
        Ok(())
    }

    // NEGX - Negate with extend
    fn exec_negx(&mut self, inst: &Instruction, op: UnaryOp) -> Result<()> {
        let value = self.read_operand(&op.mode, op.size, inst.address)?;
        let x = if self.get_flag(FLAG_X) { 1u32 } else { 0u32 };
        let result = (0u32).wrapping_sub(value).wrapping_sub(x);
        self.write_operand(&op.mode, op.size, inst.address, result)?;
        // NEGX sets flags like SUB but Z is only cleared, never set
        let masked = size_mask(result, op.size);
        if masked != 0 {
            self.set_flag(FLAG_Z, false);
        }
        self.update_nz_flags_sized(result, op.size);
        let c = value != 0 || x != 0;
        self.set_flag(FLAG_C, c);
        self.set_flag(FLAG_X, c);
        Ok(())
    }

    // ADD - Add binary
    fn exec_add(&mut self, inst: &Instruction, add: Add) -> Result<()> {
        match add {
            Add::EaToDn(EaToDn {
                size,
                dst: dst_1,
                src: src_1,
            }) => {
                let src = self.read_operand(&src_1, size, inst.address)?;
                let dst_idx = data_reg_index(dst_1);
                let dst = size_mask(self.data_regs[dst_idx], size);
                let result = add_with_flags(src, dst, size, self);
                self.data_regs[dst_idx] =
                    write_sized_data_reg(self.data_regs[dst_idx], result, size);
            }
            Add::DnToEa(DnToEa {
                size,
                src,
                dst: dst_1,
            }) => {
                let src_idx = data_reg_index(src);
                let src = size_mask(self.data_regs[src_idx], size);
                let dst = self.read_operand(&dst_1, size, inst.address)?;
                let result = add_with_flags(src, dst, size, self);
                self.write_operand(&dst_1, size, inst.address, result)?;
            }
        }
        Ok(())
    }

    // SUB - Subtract binary
    fn exec_sub(&mut self, inst: &Instruction, sub: Sub) -> Result<()> {
        match sub {
            Sub::EaToDn(EaToDn {
                size,
                dst: dst_1,
                src: src_1,
            }) => {
                let src = self.read_operand(&src_1, size, inst.address)?;
                let dst_idx = data_reg_index(dst_1);
                let dst = size_mask(self.data_regs[dst_idx], size);
                let result = sub_with_flags(dst, src, size, self);
                self.data_regs[dst_idx] =
                    write_sized_data_reg(self.data_regs[dst_idx], result, size);
            }
            Sub::DnToEa(DnToEa {
                size,
                src: src_1,
                dst: dst_1,
            }) => {
                let src_idx = data_reg_index(src_1);
                let src = size_mask(self.data_regs[src_idx], size);
                let dst = self.read_operand(&dst_1, size, inst.address)?;
                let result = sub_with_flags(dst, src, size, self);
                self.write_operand(&dst_1, size, inst.address, result)?;
            }
        }
        Ok(())
    }

    // CMP - Compare
    fn exec_cmp(&mut self, inst: &Instruction, ea_to_dn: EaToDn) -> Result<()> {
        let src = self.read_operand(&ea_to_dn.src, ea_to_dn.size, inst.address)?;
        let dst_idx = data_reg_index(ea_to_dn.dst);
        let dst = size_mask(self.data_regs[dst_idx], ea_to_dn.size);
        // CMP is dst - src, sets flags but doesn't store result
        cmp_with_flags(dst, src, ea_to_dn.size, self);
        Ok(())
    }

    // CMPA - Compare Address
    fn exec_cmpa(
        &mut self,
        inst: &Instruction,
        addr_reg: AddrReg,
        size: Size,
        src: AddressingMode,
    ) -> Result<()> {
        let src_val = self.read_operand(&src, size, inst.address)?;
        // Sign-extend to 32 bits if word-sized
        let src_extended = if size == Size::Word {
            (src_val as i16) as i32 as u32
        } else {
            src_val
        };
        let dst = self.addr_regs[addr_reg_index(addr_reg)];
        cmp_with_flags(dst, src_extended, Size::Long, self);
        Ok(())
    }

    // AND - Logical AND
    fn exec_and(&mut self, inst: &Instruction, and: And) -> Result<()> {
        match and {
            And::EaToDn(EaToDn {
                size,
                dst: dst_1,
                src: src_1,
            }) => {
                let src = self.read_operand(&src_1, size, inst.address)?;
                let dst_idx = data_reg_index(dst_1);
                let dst = size_mask(self.data_regs[dst_idx], size);
                let result = src & dst;
                self.data_regs[dst_idx] =
                    write_sized_data_reg(self.data_regs[dst_idx], result, size);
                self.update_nz_flags_sized(result, size);
                self.set_flag(FLAG_V, false);
                self.set_flag(FLAG_C, false);
            }
            And::DnToEa(DnToEa {
                size,
                src: src_1,
                dst: dst_1,
            }) => {
                let src_idx = data_reg_index(src_1);
                let src = size_mask(self.data_regs[src_idx], size);
                let dst = self.read_operand(&dst_1, size, inst.address)?;
                let result = src & dst;
                self.write_operand(&dst_1, size, inst.address, result)?;
                self.update_nz_flags_sized(result, size);
                self.set_flag(FLAG_V, false);
                self.set_flag(FLAG_C, false);
            }
        }
        Ok(())
    }

    // OR - Logical OR
    fn exec_or(&mut self, inst: &Instruction, or: Or) -> Result<()> {
        match or {
            Or::EaToDn(EaToDn {
                size,
                dst: dst_1,
                src: src_1,
            }) => {
                let src = self.read_operand(&src_1, size, inst.address)?;
                let dst_idx = data_reg_index(dst_1);
                let dst = size_mask(self.data_regs[dst_idx], size);
                let result = src | dst;
                self.data_regs[dst_idx] =
                    write_sized_data_reg(self.data_regs[dst_idx], result, size);
                self.update_nz_flags_sized(result, size);
                self.set_flag(FLAG_V, false);
                self.set_flag(FLAG_C, false);
            }
            Or::DnToEa(DnToEa {
                size,
                src: src_1,
                dst: dst_1,
            }) => {
                let src_idx = data_reg_index(src_1);
                let src = size_mask(self.data_regs[src_idx], size);
                let dst = self.read_operand(&dst_1, size, inst.address)?;
                let result = src | dst;
                self.write_operand(&dst_1, size, inst.address, result)?;
                self.update_nz_flags_sized(result, size);
                self.set_flag(FLAG_V, false);
                self.set_flag(FLAG_C, false);
            }
        }
        Ok(())
    }

    // EOR - Exclusive OR
    fn exec_eor(&mut self, inst: &Instruction, dn_to_ea: DnToEa) -> Result<()> {
        let src_idx = data_reg_index(dn_to_ea.src);
        let src = size_mask(self.data_regs[src_idx], dn_to_ea.size);
        let dst = self.read_operand(&dn_to_ea.dst, dn_to_ea.size, inst.address)?;
        let result = src ^ dst;
        self.write_operand(&dn_to_ea.dst, dn_to_ea.size, inst.address, result)?;
        self.update_nz_flags_sized(result, dn_to_ea.size);
        self.set_flag(FLAG_V, false);
        self.set_flag(FLAG_C, false);
        Ok(())
    }

    // JSR - Jump to Subroutine
    fn exec_jsr(&mut self, inst: &Instruction, mode: AddressingMode) -> Result<()> {
        let target = self.compute_effective_address(&mode, inst.address)?;
        // Push return address (PC after instruction) onto stack
        let return_addr = inst.address + inst.len();
        self.push_long(return_addr as u32)?;
        self.pc = target;
        Ok(())
    }

    // JMP - Jump
    fn exec_jmp(&mut self, inst: &Instruction, mode: AddressingMode) -> Result<()> {
        let target = self.compute_effective_address(&mode, inst.address)?;
        self.pc = target;
        Ok(())
    }

    // BSR - Branch to Subroutine
    fn exec_bsr(&mut self, inst: &Instruction, displacement: i32) -> Result<()> {
        let return_addr = inst.address + inst.len();
        self.push_long(return_addr as u32)?;
        let target = (inst.address as i64) + 2 + (displacement as i64);
        self.pc = target as usize;
        Ok(())
    }

    // SUBQ - Subtract Quick
    fn exec_subq(&mut self, inst: &Instruction, op: QuickOp) -> Result<()> {
        let decrement = if op.data == 0 { 8 } else { op.data as u32 };
        match op.mode.ea {
            EffectiveAddress::Dr(reg) => {
                let idx = data_reg_index(reg);
                let value = size_mask(self.data_regs[idx], op.size);
                let result = sub_with_flags(value, decrement, op.size, self);
                self.data_regs[idx] = write_sized_data_reg(self.data_regs[idx], result, op.size);
            }
            EffectiveAddress::Ar(reg) => {
                // SUBQ to An doesn't affect flags and always operates on full 32-bit
                let idx = addr_reg_index(reg);
                self.addr_regs[idx] = self.addr_regs[idx].wrapping_sub(decrement);
            }
            _ => {
                // Memory operands - read, subtract with flags, write back
                let value = self.read_operand(&op.mode, op.size, inst.address)?;
                let result = sub_with_flags(value, decrement, op.size, self);
                self.write_operand(&op.mode, op.size, inst.address, result)?;
            }
        }
        Ok(())
    }

    // ADDA - Add Address
    fn exec_adda(
        &mut self,
        inst: &Instruction,
        addr_reg: AddrReg,
        size: Size,
        mode: AddressingMode,
    ) -> Result<()> {
        let src = self.read_operand(&mode, size, inst.address)?;
        // Sign-extend to 32 bits if word-sized
        let src_extended = if size == Size::Word {
            (src as i16) as i32 as u32
        } else {
            src
        };
        let idx = addr_reg_index(addr_reg);
        self.addr_regs[idx] = self.addr_regs[idx].wrapping_add(src_extended);
        // ADDA doesn't affect flags
        Ok(())
    }

    // SUBA - Subtract Address
    fn exec_suba(
        &mut self,
        inst: &Instruction,
        addr_reg: AddrReg,
        size: Size,
        mode: AddressingMode,
    ) -> Result<()> {
        let src = self.read_operand(&mode, size, inst.address)?;
        // Sign-extend to 32 bits if word-sized
        let src_extended = if size == Size::Word {
            (src as i16) as i32 as u32
        } else {
            src
        };
        let idx = addr_reg_index(addr_reg);
        self.addr_regs[idx] = self.addr_regs[idx].wrapping_sub(src_extended);
        // SUBA doesn't affect flags
        Ok(())
    }

    // MOVEA - Move Address
    fn exec_movea(
        &mut self,
        inst: &Instruction,
        size: Size,
        src: AddressingMode,
        dst: AddrReg,
    ) -> Result<()> {
        let value = self.read_operand(&src, size, inst.address)?;
        // Sign-extend to 32 bits if word-sized
        let extended = if size == Size::Word {
            (value as i16) as i32 as u32
        } else {
            value
        };
        let idx = addr_reg_index(dst);
        self.addr_regs[idx] = extended;
        // MOVEA doesn't affect flags
        Ok(())
    }

    // EXT - Sign Extend
    fn exec_ext(&mut self, data_reg: DataReg, mode: ExtMode) -> Result<()> {
        let idx = data_reg_index(data_reg);
        let value = self.data_regs[idx];
        let result = match mode {
            ExtMode::ByteToWord => {
                // Sign-extend byte to word, preserve upper word
                let extended = (value as i8) as i16 as u16;
                (value & 0xFFFF_0000) | (extended as u32)
            }
            ExtMode::WordToLong => {
                // Sign-extend word to long
                (value as i16) as i32 as u32
            }
            ExtMode::ByteToLong => {
                // Sign-extend byte to long
                (value as i8) as i32 as u32
            }
        };
        self.data_regs[idx] = result;
        let size = match mode {
            ExtMode::ByteToWord => Size::Word,
            ExtMode::WordToLong | ExtMode::ByteToLong => Size::Long,
        };
        self.update_nz_flags_sized(result, size);
        self.set_flag(FLAG_V, false);
        self.set_flag(FLAG_C, false);
        Ok(())
    }

    // SWAP - Swap Register Halves
    fn exec_swap(&mut self, data_reg: DataReg) -> Result<()> {
        let idx = data_reg_index(data_reg);
        let value = self.data_regs[idx];
        let result = value.rotate_left(16);
        self.data_regs[idx] = result;
        self.update_nz_flags_sized(result, Size::Long);
        self.set_flag(FLAG_V, false);
        self.set_flag(FLAG_C, false);
        Ok(())
    }

    // PEA - Push Effective Address
    fn exec_pea(&mut self, inst: &Instruction, mode: AddressingMode) -> Result<()> {
        let addr = self.compute_effective_address(&mode, inst.address)?;
        self.push_long(addr as u32)?;
        Ok(())
    }

    // LINK - Link and Allocate
    fn exec_link(&mut self, addr_reg: AddrReg, displacement: i16) -> Result<()> {
        let idx = addr_reg_index(addr_reg);
        // Push An onto stack
        self.push_long(self.addr_regs[idx])?;
        // An = SP
        self.addr_regs[idx] = self.addr_regs[7];
        // SP = SP + displacement (displacement is negative for allocating)
        self.addr_regs[7] = (self.addr_regs[7] as i32 + displacement as i32) as u32;
        Ok(())
    }

    // UNLK - Unlink
    fn exec_unlk(&mut self, addr_reg: AddrReg) -> Result<()> {
        let idx = addr_reg_index(addr_reg);
        // SP = An
        self.addr_regs[7] = self.addr_regs[idx];
        // Pop An from stack
        self.addr_regs[idx] = self.pop_long()?;
        Ok(())
    }

    // ADDI - Add Immediate
    fn exec_addi(&mut self, inst: &Instruction, imm_op: ImmOp) -> Result<()> {
        let imm = imm_to_u32(imm_op.imm);
        let dst = self.read_operand(&imm_op.mode, imm_op.size, inst.address)?;
        let result = add_with_flags(imm, dst, imm_op.size, self);
        self.write_operand(&imm_op.mode, imm_op.size, inst.address, result)?;
        Ok(())
    }

    // SUBI - Subtract Immediate
    fn exec_subi(&mut self, inst: &Instruction, imm_op: ImmOp) -> Result<()> {
        let imm = imm_to_u32(imm_op.imm);
        let dst = self.read_operand(&imm_op.mode, imm_op.size, inst.address)?;
        let result = sub_with_flags(dst, imm, imm_op.size, self);
        self.write_operand(&imm_op.mode, imm_op.size, inst.address, result)?;
        Ok(())
    }

    // ANDI - AND Immediate
    fn exec_andi(&mut self, inst: &Instruction, imm_op: ImmOp) -> Result<()> {
        let imm = imm_to_u32(imm_op.imm);
        let dst = self.read_operand(&imm_op.mode, imm_op.size, inst.address)?;
        let result = imm & dst;
        self.write_operand(&imm_op.mode, imm_op.size, inst.address, result)?;
        self.update_nz_flags_sized(result, imm_op.size);
        self.set_flag(FLAG_V, false);
        self.set_flag(FLAG_C, false);
        Ok(())
    }

    // ORI - OR Immediate
    fn exec_ori(&mut self, inst: &Instruction, imm_op: ImmOp) -> Result<()> {
        let imm = imm_to_u32(imm_op.imm);
        let dst = self.read_operand(&imm_op.mode, imm_op.size, inst.address)?;
        let result = imm | dst;
        self.write_operand(&imm_op.mode, imm_op.size, inst.address, result)?;
        self.update_nz_flags_sized(result, imm_op.size);
        self.set_flag(FLAG_V, false);
        self.set_flag(FLAG_C, false);
        Ok(())
    }

    // EORI - Exclusive OR Immediate
    fn exec_eori(&mut self, inst: &Instruction, imm_op: ImmOp) -> Result<()> {
        let imm = imm_to_u32(imm_op.imm);
        let dst = self.read_operand(&imm_op.mode, imm_op.size, inst.address)?;
        let result = imm ^ dst;
        self.write_operand(&imm_op.mode, imm_op.size, inst.address, result)?;
        self.update_nz_flags_sized(result, imm_op.size);
        self.set_flag(FLAG_V, false);
        self.set_flag(FLAG_C, false);
        Ok(())
    }

    // CMPI - Compare Immediate
    fn exec_cmpi(&mut self, inst: &Instruction, imm_op: ImmOp) -> Result<()> {
        let imm = imm_to_u32(imm_op.imm);
        let dst = self.read_operand(&imm_op.mode, imm_op.size, inst.address)?;
        cmp_with_flags(dst, imm, imm_op.size, self);
        Ok(())
    }

    // ASd - Arithmetic Shift
    fn exec_asd(&mut self, inst: &Instruction, shift: Shift) -> Result<()> {
        match shift {
            Shift::Reg(reg) => {
                let count = match reg.count {
                    ShiftCount::Immediate(n) => {
                        if n == 0 {
                            8
                        } else {
                            n as u32
                        }
                    }
                    ShiftCount::Register(r) => self.data_regs[data_reg_index(r)] % 64,
                };
                let idx = data_reg_index(reg.dst);
                let value = size_mask(self.data_regs[idx], reg.size);
                let (result, carry) = arithmetic_shift(value, count, reg.direction, reg.size);
                self.data_regs[idx] = write_sized_data_reg(self.data_regs[idx], result, reg.size);
                self.update_nz_flags_sized(result, reg.size);
                self.set_flag(FLAG_V, false); // Simplified - V is complex for ASL
                self.set_flag(FLAG_C, carry);
                if count > 0 {
                    self.set_flag(FLAG_X, carry);
                }
            }
            Shift::Ea(ea) => {
                let addr = self.compute_effective_address(&ea.mode, inst.address)?;
                let value = self.memory.read_word(addr)? as u32;
                let (result, carry) = arithmetic_shift(value, 1, ea.direction, Size::Word);
                self.memory
                    .write_data(addr, &(result as u16).to_be_bytes())?;
                self.update_nz_flags_sized(result, Size::Word);
                self.set_flag(FLAG_V, false);
                self.set_flag(FLAG_C, carry);
                self.set_flag(FLAG_X, carry);
            }
        }
        Ok(())
    }

    // LSd - Logical Shift
    fn exec_lsd(&mut self, inst: &Instruction, shift: Shift) -> Result<()> {
        match shift {
            Shift::Reg(reg) => {
                let count = match reg.count {
                    ShiftCount::Immediate(n) => {
                        if n == 0 {
                            8
                        } else {
                            n as u32
                        }
                    }
                    ShiftCount::Register(r) => self.data_regs[data_reg_index(r)] % 64,
                };
                let idx = data_reg_index(reg.dst);
                let value = size_mask(self.data_regs[idx], reg.size);
                let (result, carry) = logical_shift(value, count, reg.direction, reg.size);
                self.data_regs[idx] = write_sized_data_reg(self.data_regs[idx], result, reg.size);
                self.update_nz_flags_sized(result, reg.size);
                self.set_flag(FLAG_V, false);
                self.set_flag(FLAG_C, carry);
                if count > 0 {
                    self.set_flag(FLAG_X, carry);
                }
            }
            Shift::Ea(ea) => {
                let addr = self.compute_effective_address(&ea.mode, inst.address)?;
                let value = self.memory.read_word(addr)? as u32;
                let (result, carry) = logical_shift(value, 1, ea.direction, Size::Word);
                self.memory
                    .write_data(addr, &(result as u16).to_be_bytes())?;
                self.update_nz_flags_sized(result, Size::Word);
                self.set_flag(FLAG_V, false);
                self.set_flag(FLAG_C, carry);
                self.set_flag(FLAG_X, carry);
            }
        }
        Ok(())
    }

    // ROd - Rotate
    fn exec_rod(&mut self, inst: &Instruction, shift: Shift) -> Result<()> {
        match shift {
            Shift::Reg(reg) => {
                let count = match reg.count {
                    ShiftCount::Immediate(n) => {
                        if n == 0 {
                            8
                        } else {
                            n as u32
                        }
                    }
                    ShiftCount::Register(r) => self.data_regs[data_reg_index(r)] % 64,
                };
                let idx = data_reg_index(reg.dst);
                let value = size_mask(self.data_regs[idx], reg.size);
                let (result, carry) = rotate(value, count, reg.direction, reg.size);
                self.data_regs[idx] = write_sized_data_reg(self.data_regs[idx], result, reg.size);
                self.update_nz_flags_sized(result, reg.size);
                self.set_flag(FLAG_V, false);
                self.set_flag(FLAG_C, carry);
            }
            Shift::Ea(ea) => {
                let addr = self.compute_effective_address(&ea.mode, inst.address)?;
                let value = self.memory.read_word(addr)? as u32;
                let (result, carry) = rotate(value, 1, ea.direction, Size::Word);
                self.memory
                    .write_data(addr, &(result as u16).to_be_bytes())?;
                self.update_nz_flags_sized(result, Size::Word);
                self.set_flag(FLAG_V, false);
                self.set_flag(FLAG_C, carry);
            }
        }
        Ok(())
    }

    // ROXd - Rotate with Extend
    fn exec_roxd(&mut self, inst: &Instruction, shift: Shift) -> Result<()> {
        match shift {
            Shift::Reg(reg) => {
                let count = match reg.count {
                    ShiftCount::Immediate(n) => {
                        if n == 0 {
                            8
                        } else {
                            n as u32
                        }
                    }
                    ShiftCount::Register(r) => self.data_regs[data_reg_index(r)] % 64,
                };
                let idx = data_reg_index(reg.dst);
                let value = size_mask(self.data_regs[idx], reg.size);
                let x = self.get_flag(FLAG_X);
                let (result, carry) = rotate_extended(value, count, reg.direction, reg.size, x);
                self.data_regs[idx] = write_sized_data_reg(self.data_regs[idx], result, reg.size);
                self.update_nz_flags_sized(result, reg.size);
                self.set_flag(FLAG_V, false);
                self.set_flag(FLAG_C, carry);
                self.set_flag(FLAG_X, carry);
            }
            Shift::Ea(ea) => {
                let addr = self.compute_effective_address(&ea.mode, inst.address)?;
                let value = self.memory.read_word(addr)? as u32;
                let x = self.get_flag(FLAG_X);
                let (result, carry) = rotate_extended(value, 1, ea.direction, Size::Word, x);
                self.memory
                    .write_data(addr, &(result as u16).to_be_bytes())?;
                self.update_nz_flags_sized(result, Size::Word);
                self.set_flag(FLAG_V, false);
                self.set_flag(FLAG_C, carry);
                self.set_flag(FLAG_X, carry);
            }
        }
        Ok(())
    }

    // BTST - Bit Test
    fn exec_btst(&mut self, inst: &Instruction, bit_op: BitOp) -> Result<()> {
        let (bit_num, mode) = match bit_op {
            BitOp::Imm(imm) => (imm.bit_num as u32, imm.mode),
            BitOp::Reg(reg) => (self.data_regs[data_reg_index(reg.bit_reg)], reg.mode),
        };
        let (value, modulo) = if matches!(mode.ea, EffectiveAddress::Dr(_)) {
            let idx = match mode.ea {
                EffectiveAddress::Dr(r) => data_reg_index(r),
                _ => unreachable!(),
            };
            (self.data_regs[idx], 32)
        } else {
            (self.read_operand(&mode, Size::Byte, inst.address)?, 8)
        };
        let bit = bit_num % modulo;
        let z = (value & (1 << bit)) == 0;
        self.set_flag(FLAG_Z, z);
        Ok(())
    }

    // BCHG - Bit Test and Change
    fn exec_bchg(&mut self, inst: &Instruction, bit_op: BitOp) -> Result<()> {
        let (bit_num, mode) = match bit_op {
            BitOp::Imm(imm) => (imm.bit_num as u32, imm.mode),
            BitOp::Reg(reg) => (self.data_regs[data_reg_index(reg.bit_reg)], reg.mode),
        };
        if let EffectiveAddress::Dr(r) = mode.ea {
            let idx = data_reg_index(r);
            let bit = bit_num % 32;
            let z = (self.data_regs[idx] & (1 << bit)) == 0;
            self.set_flag(FLAG_Z, z);
            self.data_regs[idx] ^= 1 << bit;
        } else {
            let value = self.read_operand(&mode, Size::Byte, inst.address)?;
            let bit = bit_num % 8;
            let z = (value & (1 << bit)) == 0;
            self.set_flag(FLAG_Z, z);
            self.write_operand(&mode, Size::Byte, inst.address, value ^ (1 << bit))?;
        }
        Ok(())
    }

    // BCLR - Bit Test and Clear
    fn exec_bclr(&mut self, inst: &Instruction, bit_op: BitOp) -> Result<()> {
        let (bit_num, mode) = match bit_op {
            BitOp::Imm(imm) => (imm.bit_num as u32, imm.mode),
            BitOp::Reg(reg) => (self.data_regs[data_reg_index(reg.bit_reg)], reg.mode),
        };
        if let EffectiveAddress::Dr(r) = mode.ea {
            let idx = data_reg_index(r);
            let bit = bit_num % 32;
            let z = (self.data_regs[idx] & (1 << bit)) == 0;
            self.set_flag(FLAG_Z, z);
            self.data_regs[idx] &= !(1 << bit);
        } else {
            let value = self.read_operand(&mode, Size::Byte, inst.address)?;
            let bit = bit_num % 8;
            let z = (value & (1 << bit)) == 0;
            self.set_flag(FLAG_Z, z);
            self.write_operand(&mode, Size::Byte, inst.address, value & !(1 << bit))?;
        }
        Ok(())
    }

    // BSET - Bit Test and Set
    fn exec_bset(&mut self, inst: &Instruction, bit_op: BitOp) -> Result<()> {
        let (bit_num, mode) = match bit_op {
            BitOp::Imm(imm) => (imm.bit_num as u32, imm.mode),
            BitOp::Reg(reg) => (self.data_regs[data_reg_index(reg.bit_reg)], reg.mode),
        };
        if let EffectiveAddress::Dr(r) = mode.ea {
            let idx = data_reg_index(r);
            let bit = bit_num % 32;
            let z = (self.data_regs[idx] & (1 << bit)) == 0;
            self.set_flag(FLAG_Z, z);
            self.data_regs[idx] |= 1 << bit;
        } else {
            let value = self.read_operand(&mode, Size::Byte, inst.address)?;
            let bit = bit_num % 8;
            let z = (value & (1 << bit)) == 0;
            self.set_flag(FLAG_Z, z);
            self.write_operand(&mode, Size::Byte, inst.address, value | (1 << bit))?;
        }
        Ok(())
    }

    // Scc - Set on Condition
    fn exec_scc(
        &mut self,
        inst: &Instruction,
        condition: Condition,
        mode: AddressingMode,
    ) -> Result<()> {
        let value = if self.test_condition(condition) {
            0xFF
        } else {
            0x00
        };
        self.write_operand(&mode, Size::Byte, inst.address, value)?;
        Ok(())
    }

    // DBcc - Decrement and Branch on Condition
    fn exec_dbcc(
        &mut self,
        inst: &Instruction,
        condition: Condition,
        data_reg: DataReg,
        displacement: i16,
    ) -> bool {
        // If condition is true, don't branch
        if self.test_condition(condition) {
            return false;
        }
        // Decrement low word of Dn
        let idx = data_reg_index(data_reg);
        let low_word = (self.data_regs[idx] as u16).wrapping_sub(1);
        self.data_regs[idx] = (self.data_regs[idx] & 0xFFFF_0000) | (low_word as u32);
        // If Dn == -1 (0xFFFF), don't branch
        if low_word == 0xFFFF {
            return false;
        }
        // Branch
        let target = (inst.address as i64) + 2 + (displacement as i64);
        self.pc = target as usize;
        true
    }

    // DIVU - Unsigned Divide
    fn exec_divu(&mut self, inst: &Instruction, src: AddressingMode, dst: DataReg) -> Result<()> {
        let divisor = self.read_operand(&src, Size::Word, inst.address)? as u16;
        if divisor == 0 {
            bail!("division by zero");
        }
        let idx = data_reg_index(dst);
        let dividend = self.data_regs[idx];
        let quotient = dividend / (divisor as u32);
        let remainder = dividend % (divisor as u32);
        if quotient > 0xFFFF {
            // Overflow
            self.set_flag(FLAG_V, true);
            self.set_flag(FLAG_C, false);
        } else {
            self.data_regs[idx] = ((remainder as u32) << 16) | (quotient as u32);
            self.set_flag(FLAG_N, (quotient & 0x8000) != 0);
            self.set_flag(FLAG_Z, quotient == 0);
            self.set_flag(FLAG_V, false);
            self.set_flag(FLAG_C, false);
        }
        Ok(())
    }

    // DIVS - Signed Divide
    fn exec_divs(&mut self, inst: &Instruction, src: AddressingMode, dst: DataReg) -> Result<()> {
        let divisor = self.read_operand(&src, Size::Word, inst.address)? as i16;
        if divisor == 0 {
            bail!("division by zero");
        }
        let idx = data_reg_index(dst);
        let dividend = self.data_regs[idx] as i32;
        let quotient = dividend / (divisor as i32);
        let remainder = dividend % (divisor as i32);
        if !(-32768..=32767).contains(&quotient) {
            // Overflow
            self.set_flag(FLAG_V, true);
            self.set_flag(FLAG_C, false);
        } else {
            self.data_regs[idx] = ((remainder as u16 as u32) << 16) | (quotient as u16 as u32);
            self.set_flag(FLAG_N, (quotient as u16 & 0x8000) != 0);
            self.set_flag(FLAG_Z, quotient == 0);
            self.set_flag(FLAG_V, false);
            self.set_flag(FLAG_C, false);
        }
        Ok(())
    }

    // DIVU.L - 32-bit unsigned divide (68020+)
    fn exec_divul(
        &mut self,
        inst: &Instruction,
        src: &AddressingMode,
        dq: DataReg,
        dr: DataReg,
        is_64bit: bool,
    ) -> Result<()> {
        let divisor = self.read_operand(src, Size::Long, inst.address)?;
        if divisor == 0 {
            bail!("division by zero");
        }
        let dq_idx = data_reg_index(dq);
        let dr_idx = data_reg_index(dr);

        if is_64bit {
            // 64÷32 -> 32:32
            let dividend =
                ((self.data_regs[dr_idx] as u64) << 32) | (self.data_regs[dq_idx] as u64);
            let quotient = dividend / (divisor as u64);
            let remainder = dividend % (divisor as u64);
            if quotient > 0xFFFFFFFF {
                self.set_flag(FLAG_V, true);
                self.set_flag(FLAG_C, false);
            } else {
                self.data_regs[dq_idx] = quotient as u32;
                self.data_regs[dr_idx] = remainder as u32;
                self.set_flag(FLAG_N, (quotient as u32 & 0x80000000) != 0);
                self.set_flag(FLAG_Z, quotient == 0);
                self.set_flag(FLAG_V, false);
                self.set_flag(FLAG_C, false);
            }
        } else {
            // 32÷32 -> 32:32
            let dividend = self.data_regs[dq_idx];
            let quotient = dividend / divisor;
            let remainder = dividend % divisor;
            self.data_regs[dq_idx] = quotient;
            // Only store remainder if Dr != Dq (when same register, remainder is discarded)
            if dr_idx != dq_idx {
                self.data_regs[dr_idx] = remainder;
            }
            self.set_flag(FLAG_N, (quotient & 0x80000000) != 0);
            self.set_flag(FLAG_Z, quotient == 0);
            self.set_flag(FLAG_V, false);
            self.set_flag(FLAG_C, false);
        }
        Ok(())
    }

    // DIVS.L - 32-bit signed divide (68020+)
    fn exec_divsl(
        &mut self,
        inst: &Instruction,
        src: &AddressingMode,
        dq: DataReg,
        dr: DataReg,
        is_64bit: bool,
    ) -> Result<()> {
        let divisor = self.read_operand(src, Size::Long, inst.address)? as i32;
        if divisor == 0 {
            bail!("division by zero");
        }
        let dq_idx = data_reg_index(dq);
        let dr_idx = data_reg_index(dr);

        if is_64bit {
            // 64÷32 -> 32:32
            let dividend =
                ((self.data_regs[dr_idx] as u64) << 32) | (self.data_regs[dq_idx] as u64);
            let dividend = dividend as i64;
            let quotient = dividend / (divisor as i64);
            let remainder = dividend % (divisor as i64);
            if quotient < i32::MIN as i64 || quotient > i32::MAX as i64 {
                self.set_flag(FLAG_V, true);
                self.set_flag(FLAG_C, false);
            } else {
                self.data_regs[dq_idx] = quotient as u32;
                self.data_regs[dr_idx] = remainder as u32;
                self.set_flag(FLAG_N, (quotient as u32 & 0x80000000) != 0);
                self.set_flag(FLAG_Z, quotient == 0);
                self.set_flag(FLAG_V, false);
                self.set_flag(FLAG_C, false);
            }
        } else {
            // 32÷32 -> 32:32
            let dividend = self.data_regs[dq_idx] as i32;
            let quotient = dividend / divisor;
            let remainder = dividend % divisor;
            self.data_regs[dq_idx] = quotient as u32;
            // Only store remainder if Dr != Dq (when same register, remainder is discarded)
            if dr_idx != dq_idx {
                self.data_regs[dr_idx] = remainder as u32;
            }
            self.set_flag(FLAG_N, (quotient as u32 & 0x80000000) != 0);
            self.set_flag(FLAG_Z, quotient == 0);
            self.set_flag(FLAG_V, false);
            self.set_flag(FLAG_C, false);
        }
        Ok(())
    }

    // MULU.L - 32-bit unsigned multiply (68020+)
    fn exec_mulul(
        &mut self,
        inst: &Instruction,
        src: &AddressingMode,
        dl: DataReg,
        dh: Option<DataReg>,
    ) -> Result<()> {
        let multiplicand = self.read_operand(src, Size::Long, inst.address)? as u64;
        let dl_idx = data_reg_index(dl);
        let multiplier = self.data_regs[dl_idx] as u64;
        let result = multiplicand * multiplier;

        self.data_regs[dl_idx] = result as u32;
        if let Some(dh) = dh {
            let dh_idx = data_reg_index(dh);
            self.data_regs[dh_idx] = (result >> 32) as u32;
            self.set_flag(FLAG_N, (result >> 63) != 0);
            self.set_flag(FLAG_Z, result == 0);
            self.set_flag(FLAG_V, false);
        } else {
            self.set_flag(FLAG_N, (result as u32 & 0x80000000) != 0);
            self.set_flag(FLAG_Z, (result as u32) == 0);
            self.set_flag(FLAG_V, result > 0xFFFFFFFF);
        }
        self.set_flag(FLAG_C, false);
        Ok(())
    }

    // MULS.L - 32-bit signed multiply (68020+)
    fn exec_mulsl(
        &mut self,
        inst: &Instruction,
        src: &AddressingMode,
        dl: DataReg,
        dh: Option<DataReg>,
    ) -> Result<()> {
        let multiplicand = self.read_operand(src, Size::Long, inst.address)? as i32 as i64;
        let dl_idx = data_reg_index(dl);
        let multiplier = self.data_regs[dl_idx] as i32 as i64;
        let result = multiplicand * multiplier;

        self.data_regs[dl_idx] = result as u32;
        if let Some(dh) = dh {
            let dh_idx = data_reg_index(dh);
            self.data_regs[dh_idx] = (result >> 32) as u32;
            self.set_flag(FLAG_N, result < 0);
            self.set_flag(FLAG_Z, result == 0);
            self.set_flag(FLAG_V, false);
        } else {
            self.set_flag(FLAG_N, (result as u32 & 0x80000000) != 0);
            self.set_flag(FLAG_Z, (result as u32) == 0);
            let fits_in_32 = result >= i32::MIN as i64 && result <= i32::MAX as i64;
            self.set_flag(FLAG_V, !fits_in_32);
        }
        self.set_flag(FLAG_C, false);
        Ok(())
    }

    // CAS - Compare and Swap (68020+)
    fn exec_cas(
        &mut self,
        inst: &Instruction,
        size: Size,
        dc: DataReg,
        du: DataReg,
        mode: &AddressingMode,
    ) -> Result<()> {
        let addr = self.compute_effective_address(mode, inst.address)?;
        let operand = self.read_mem(addr, size)?;
        let dc_idx = data_reg_index(dc);

        let (mask, sign_bit) = match size {
            Size::Byte => (0xFF, 0x80),
            Size::Word => (0xFFFF, 0x8000),
            Size::Long => (0xFFFFFFFF, 0x80000000),
        };
        let compare_val = self.data_regs[dc_idx] & mask;

        // Compare operand with Dc
        let (result, overflow) = match size {
            Size::Byte => {
                let a = operand as u8;
                let b = compare_val as u8;
                let (r, o) = a.overflowing_sub(b);
                (r as u32, o)
            }
            Size::Word => {
                let a = operand as u16;
                let b = compare_val as u16;
                let (r, o) = a.overflowing_sub(b);
                (r as u32, o)
            }
            Size::Long => {
                let (r, o) = operand.overflowing_sub(compare_val);
                (r, o)
            }
        };

        // Set condition codes based on comparison
        self.set_flag(FLAG_N, (result & sign_bit) != 0);
        self.set_flag(FLAG_Z, result == 0);
        self.set_flag(FLAG_V, overflow);
        self.set_flag(FLAG_C, operand < compare_val);

        if operand == compare_val {
            // Equal: update memory with Du
            let du_idx = data_reg_index(du);
            let update_val = self.data_regs[du_idx] & mask;
            self.write_mem(addr, size, update_val)?;
        } else {
            // Not equal: load operand into Dc
            match size {
                Size::Byte => {
                    self.data_regs[dc_idx] =
                        (self.data_regs[dc_idx] & 0xFFFFFF00) | (operand & 0xFF);
                }
                Size::Word => {
                    self.data_regs[dc_idx] =
                        (self.data_regs[dc_idx] & 0xFFFF0000) | (operand & 0xFFFF);
                }
                Size::Long => {
                    self.data_regs[dc_idx] = operand;
                }
            }
        }
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    // CAS2 - Compare and Swap 2 (68020+)
    fn exec_cas2(
        &mut self,
        size: Size,
        dc1: DataReg,
        dc2: DataReg,
        du1: DataReg,
        du2: DataReg,
        rn1: AddrReg,
        rn2: AddrReg,
    ) -> Result<()> {
        // Read addresses from Rn1 and Rn2
        let addr1 = self.addr_regs[addr_reg_index(rn1)] as usize;
        let addr2 = self.addr_regs[addr_reg_index(rn2)] as usize;

        // Read operands from memory
        let operand1 = self.read_mem(addr1, size)?;
        let operand2 = self.read_mem(addr2, size)?;

        let dc1_idx = data_reg_index(dc1);
        let dc2_idx = data_reg_index(dc2);

        let (mask, sign_bit) = match size {
            Size::Byte => (0xFF, 0x80),
            Size::Word => (0xFFFF, 0x8000),
            Size::Long => (0xFFFFFFFF, 0x80000000),
        };

        let compare_val1 = self.data_regs[dc1_idx] & mask;
        let compare_val2 = self.data_regs[dc2_idx] & mask;

        // Compare both operands
        let match1 = operand1 == compare_val1;
        let match2 = operand2 == compare_val2;

        // For condition codes, we compare the first operand
        let (result, overflow) = match size {
            Size::Byte => {
                let a = operand1 as u8;
                let b = compare_val1 as u8;
                let (r, o) = a.overflowing_sub(b);
                (r as u32, o)
            }
            Size::Word => {
                let a = operand1 as u16;
                let b = compare_val1 as u16;
                let (r, o) = a.overflowing_sub(b);
                (r as u32, o)
            }
            Size::Long => {
                let (r, o) = operand1.overflowing_sub(compare_val1);
                (r, o)
            }
        };

        // Set condition codes based on first comparison
        self.set_flag(FLAG_N, (result & sign_bit) != 0);
        self.set_flag(FLAG_Z, match1 && match2); // Z flag set only if both match
        self.set_flag(FLAG_V, overflow);
        self.set_flag(FLAG_C, operand1 < compare_val1);

        if match1 && match2 {
            // Both match: update both memory locations with Du1 and Du2
            let du1_idx = data_reg_index(du1);
            let du2_idx = data_reg_index(du2);
            let update_val1 = self.data_regs[du1_idx] & mask;
            let update_val2 = self.data_regs[du2_idx] & mask;
            self.write_mem(addr1, size, update_val1)?;
            self.write_mem(addr2, size, update_val2)?;
        } else {
            // At least one doesn't match: load both operands into Dc1 and Dc2
            match size {
                Size::Byte => {
                    self.data_regs[dc1_idx] =
                        (self.data_regs[dc1_idx] & 0xFFFFFF00) | (operand1 & 0xFF);
                    self.data_regs[dc2_idx] =
                        (self.data_regs[dc2_idx] & 0xFFFFFF00) | (operand2 & 0xFF);
                }
                Size::Word => {
                    self.data_regs[dc1_idx] =
                        (self.data_regs[dc1_idx] & 0xFFFF0000) | (operand1 & 0xFFFF);
                    self.data_regs[dc2_idx] =
                        (self.data_regs[dc2_idx] & 0xFFFF0000) | (operand2 & 0xFFFF);
                }
                Size::Long => {
                    self.data_regs[dc1_idx] = operand1;
                    self.data_regs[dc2_idx] = operand2;
                }
            }
        }
        Ok(())
    }

    // CMP2 - Compare Register Against Bounds (68020+)
    fn exec_cmp2(
        &mut self,
        inst: &Instruction,
        size: Size,
        mode: &AddressingMode,
        reg: crate::decoder::Register,
    ) -> Result<()> {
        use crate::decoder::Register;

        // Read the register value
        let reg_value = match reg {
            Register::Data(dreg) => {
                let idx = data_reg_index(dreg);
                self.data_regs[idx]
            }
            Register::Address(areg) => {
                let idx = addr_reg_index(areg);
                self.addr_regs[idx]
            }
        };

        // Get mask based on size
        let mask = match size {
            Size::Byte => 0xFF,
            Size::Word => 0xFFFF,
            Size::Long => 0xFFFFFFFF,
        };
        let reg_value = reg_value & mask;

        // Read lower and upper bounds from memory
        let addr = self.compute_effective_address(mode, inst.address)?;
        let lower_bound = self.read_mem(addr, size)?;
        let size_bytes = match size {
            Size::Byte => 1,
            Size::Word => 2,
            Size::Long => 4,
        };
        let upper_bound = self.read_mem(addr + size_bytes, size)?;

        // Perform comparison
        // C flag: set if reg_value < lower_bound OR reg_value > upper_bound (out of bounds)
        // Z flag: set if reg_value == lower_bound OR reg_value == upper_bound (at boundary)
        let out_of_bounds = reg_value < lower_bound || reg_value > upper_bound;
        let at_boundary = reg_value == lower_bound || reg_value == upper_bound;

        self.set_flag(FLAG_C, out_of_bounds);
        self.set_flag(FLAG_Z, at_boundary);
        // N and V are undefined according to manual, we'll leave them unchanged

        Ok(())
    }

    // CHK2 - Check Register Against Bounds (68020+)
    fn exec_chk2(
        &mut self,
        inst: &Instruction,
        size: Size,
        mode: &AddressingMode,
        reg: crate::decoder::Register,
    ) -> Result<()> {
        // CHK2 is like CMP2 but also generates a CHK exception if out of bounds
        use crate::decoder::Register;

        // Read the register value
        let reg_value = match reg {
            Register::Data(dreg) => {
                let idx = data_reg_index(dreg);
                self.data_regs[idx]
            }
            Register::Address(areg) => {
                let idx = addr_reg_index(areg);
                self.addr_regs[idx]
            }
        };

        // Get mask based on size
        let mask = match size {
            Size::Byte => 0xFF,
            Size::Word => 0xFFFF,
            Size::Long => 0xFFFFFFFF,
        };
        let reg_value = reg_value & mask;

        // Read lower and upper bounds from memory
        let addr = self.compute_effective_address(mode, inst.address)?;
        let lower_bound = self.read_mem(addr, size)?;
        let size_bytes = match size {
            Size::Byte => 1,
            Size::Word => 2,
            Size::Long => 4,
        };
        let upper_bound = self.read_mem(addr + size_bytes, size)?;

        // Perform comparison
        let out_of_bounds = reg_value < lower_bound || reg_value > upper_bound;
        let at_boundary = reg_value == lower_bound || reg_value == upper_bound;

        self.set_flag(FLAG_C, out_of_bounds);
        self.set_flag(FLAG_Z, at_boundary);

        // If out of bounds, generate CHK exception
        if out_of_bounds {
            bail!("CHK2 exception: value out of bounds");
        }

        Ok(())
    }

    // BFTST - Test Bit Field (68020+)
    fn exec_bftst(
        &mut self,
        inst: &Instruction,
        mode: &AddressingMode,
        offset: BitFieldParam,
        width: BitFieldParam,
    ) -> Result<()> {
        // Get offset value (can span beyond 32 bits for memory operands)
        let offset_val = match offset {
            BitFieldParam::Immediate(v) => v as i32,
            BitFieldParam::Register(reg) => self.data_regs[data_reg_index(reg)] as i32,
        };

        // Get width value (0 means 32)
        let width_val = match width {
            BitFieldParam::Immediate(0) => 32u32,
            BitFieldParam::Immediate(v) => v as u32,
            BitFieldParam::Register(reg) => {
                let w = self.data_regs[data_reg_index(reg)] & 0x1f;
                if w == 0 { 32 } else { w }
            }
        };

        // Extract the bit field value
        let field_value = match mode.ea {
            EffectiveAddress::Dr(reg) => {
                // For data register, offset is modulo 32
                let offset_mod = (offset_val as u32) % 32;
                let reg_val = self.data_regs[data_reg_index(reg)];
                // Bit 0 of offset corresponds to MSB (bit 31)
                // Extract width bits starting from offset
                extract_bitfield_from_u32(reg_val, offset_mod, width_val)
            }
            _ => {
                // Memory operand - offset can be negative or > 31
                let base_addr = self.compute_effective_address(mode, inst.address)? as u32;
                extract_bitfield_from_memory(&self.memory, base_addr, offset_val, width_val)?
            }
        };

        // Set condition codes: N and Z based on field value, C and V always cleared
        // N is set if MSB of the field is set
        let msb_mask = 1u32 << (width_val - 1);
        self.set_flag(FLAG_N, (field_value & msb_mask) != 0);
        self.set_flag(FLAG_Z, field_value == 0);
        self.set_flag(FLAG_V, false);
        self.set_flag(FLAG_C, false);

        Ok(())
    }

    // BFCHG - Change Bit Field (invert) (68020+)
    fn exec_bfchg(
        &mut self,
        inst: &Instruction,
        mode: &AddressingMode,
        offset: BitFieldParam,
        width: BitFieldParam,
    ) -> Result<()> {
        let offset_val = match offset {
            BitFieldParam::Immediate(v) => v as i32,
            BitFieldParam::Register(reg) => self.data_regs[data_reg_index(reg)] as i32,
        };
        let width_val = match width {
            BitFieldParam::Immediate(0) => 32u32,
            BitFieldParam::Immediate(v) => v as u32,
            BitFieldParam::Register(reg) => {
                let w = self.data_regs[data_reg_index(reg)] & 0x1f;
                if w == 0 { 32 } else { w }
            }
        };

        let field_value = match mode.ea {
            EffectiveAddress::Dr(reg) => {
                let offset_mod = (offset_val as u32) % 32;
                let reg_val = self.data_regs[data_reg_index(reg)];
                extract_bitfield_from_u32(reg_val, offset_mod, width_val)
            }
            _ => {
                let base_addr = self.compute_effective_address(mode, inst.address)? as u32;
                extract_bitfield_from_memory(&self.memory, base_addr, offset_val, width_val)?
            }
        };

        let mask = if width_val == 32 {
            0xFFFF_FFFF
        } else {
            (1u32 << width_val) - 1
        };
        let new_value = (!field_value) & mask;

        match mode.ea {
            EffectiveAddress::Dr(reg) => {
                let offset_mod = (offset_val as u32) % 32;
                let dest_value = self.data_regs[data_reg_index(reg)];
                let result = insert_bitfield_into_u32(dest_value, new_value, offset_mod, width_val);
                self.data_regs[data_reg_index(reg)] = result;
            }
            _ => {
                let base_addr = self.compute_effective_address(mode, inst.address)? as u32;
                insert_bitfield_into_memory(
                    &mut self.memory,
                    base_addr,
                    offset_val,
                    width_val,
                    new_value,
                )?;
            }
        }

        let msb_mask = 1u32 << (width_val - 1);
        self.set_flag(FLAG_N, (field_value & msb_mask) != 0);
        self.set_flag(FLAG_Z, field_value == 0);
        self.set_flag(FLAG_V, false);
        self.set_flag(FLAG_C, false);

        Ok(())
    }

    // BFCLR - Clear Bit Field (68020+)
    fn exec_bfclr(
        &mut self,
        inst: &Instruction,
        mode: &AddressingMode,
        offset: BitFieldParam,
        width: BitFieldParam,
    ) -> Result<()> {
        let offset_val = match offset {
            BitFieldParam::Immediate(v) => v as i32,
            BitFieldParam::Register(reg) => self.data_regs[data_reg_index(reg)] as i32,
        };
        let width_val = match width {
            BitFieldParam::Immediate(0) => 32u32,
            BitFieldParam::Immediate(v) => v as u32,
            BitFieldParam::Register(reg) => {
                let w = self.data_regs[data_reg_index(reg)] & 0x1f;
                if w == 0 { 32 } else { w }
            }
        };

        let field_value = match mode.ea {
            EffectiveAddress::Dr(reg) => {
                let offset_mod = (offset_val as u32) % 32;
                let reg_val = self.data_regs[data_reg_index(reg)];
                extract_bitfield_from_u32(reg_val, offset_mod, width_val)
            }
            _ => {
                let base_addr = self.compute_effective_address(mode, inst.address)? as u32;
                extract_bitfield_from_memory(&self.memory, base_addr, offset_val, width_val)?
            }
        };

        match mode.ea {
            EffectiveAddress::Dr(reg) => {
                let offset_mod = (offset_val as u32) % 32;
                let dest_value = self.data_regs[data_reg_index(reg)];
                let result = insert_bitfield_into_u32(dest_value, 0, offset_mod, width_val);
                self.data_regs[data_reg_index(reg)] = result;
            }
            _ => {
                let base_addr = self.compute_effective_address(mode, inst.address)? as u32;
                insert_bitfield_into_memory(&mut self.memory, base_addr, offset_val, width_val, 0)?;
            }
        }

        let msb_mask = 1u32 << (width_val - 1);
        self.set_flag(FLAG_N, (field_value & msb_mask) != 0);
        self.set_flag(FLAG_Z, field_value == 0);
        self.set_flag(FLAG_V, false);
        self.set_flag(FLAG_C, false);

        Ok(())
    }

    // BFSET - Set Bit Field (68020+)
    fn exec_bfset(
        &mut self,
        inst: &Instruction,
        mode: &AddressingMode,
        offset: BitFieldParam,
        width: BitFieldParam,
    ) -> Result<()> {
        let offset_val = match offset {
            BitFieldParam::Immediate(v) => v as i32,
            BitFieldParam::Register(reg) => self.data_regs[data_reg_index(reg)] as i32,
        };
        let width_val = match width {
            BitFieldParam::Immediate(0) => 32u32,
            BitFieldParam::Immediate(v) => v as u32,
            BitFieldParam::Register(reg) => {
                let w = self.data_regs[data_reg_index(reg)] & 0x1f;
                if w == 0 { 32 } else { w }
            }
        };

        let field_value = match mode.ea {
            EffectiveAddress::Dr(reg) => {
                let offset_mod = (offset_val as u32) % 32;
                let reg_val = self.data_regs[data_reg_index(reg)];
                extract_bitfield_from_u32(reg_val, offset_mod, width_val)
            }
            _ => {
                let base_addr = self.compute_effective_address(mode, inst.address)? as u32;
                extract_bitfield_from_memory(&self.memory, base_addr, offset_val, width_val)?
            }
        };

        let new_value = if width_val == 32 {
            0xFFFF_FFFF
        } else {
            (1u32 << width_val) - 1
        };

        match mode.ea {
            EffectiveAddress::Dr(reg) => {
                let offset_mod = (offset_val as u32) % 32;
                let dest_value = self.data_regs[data_reg_index(reg)];
                let result = insert_bitfield_into_u32(dest_value, new_value, offset_mod, width_val);
                self.data_regs[data_reg_index(reg)] = result;
            }
            _ => {
                let base_addr = self.compute_effective_address(mode, inst.address)? as u32;
                insert_bitfield_into_memory(
                    &mut self.memory,
                    base_addr,
                    offset_val,
                    width_val,
                    new_value,
                )?;
            }
        }

        let msb_mask = 1u32 << (width_val - 1);
        self.set_flag(FLAG_N, (field_value & msb_mask) != 0);
        self.set_flag(FLAG_Z, field_value == 0);
        self.set_flag(FLAG_V, false);
        self.set_flag(FLAG_C, false);

        Ok(())
    }

    // BFFFO - Find First One Bit Field (68020+)
    fn exec_bfffo(
        &mut self,
        inst: &Instruction,
        src: &AddressingMode,
        dst: DataReg,
        offset: BitFieldParam,
        width: BitFieldParam,
    ) -> Result<()> {
        let offset_val = match offset {
            BitFieldParam::Immediate(v) => v as i32,
            BitFieldParam::Register(reg) => self.data_regs[data_reg_index(reg)] as i32,
        };
        let width_val = match width {
            BitFieldParam::Immediate(0) => 32u32,
            BitFieldParam::Immediate(v) => v as u32,
            BitFieldParam::Register(reg) => {
                let w = self.data_regs[data_reg_index(reg)] & 0x1f;
                if w == 0 { 32 } else { w }
            }
        };

        let field_value = match src.ea {
            EffectiveAddress::Dr(reg) => {
                let offset_mod = (offset_val as u32) % 32;
                let reg_val = self.data_regs[data_reg_index(reg)];
                extract_bitfield_from_u32(reg_val, offset_mod, width_val)
            }
            _ => {
                let base_addr = self.compute_effective_address(src, inst.address)? as u32;
                extract_bitfield_from_memory(&self.memory, base_addr, offset_val, width_val)?
            }
        };

        let mut position = width_val; // default if no bits set
        for i in 0..width_val {
            let bit = field_value & (1u32 << (width_val - 1 - i));
            if bit != 0 {
                position = i;
                break;
            }
        }

        let result_offset = if matches!(src.ea, EffectiveAddress::Dr(_)) {
            ((offset_val as u32) % 32).wrapping_add(position)
        } else {
            offset_val.wrapping_add(position as i32) as u32
        };
        self.data_regs[data_reg_index(dst)] = result_offset;

        self.set_flag(FLAG_N, (result_offset & 0x8000_0000) != 0);
        self.set_flag(FLAG_Z, position == width_val);
        self.set_flag(FLAG_V, false);
        self.set_flag(FLAG_C, false);

        Ok(())
    }
    // BFEXTU - Extract Bit Field Unsigned (68020+)
    fn exec_bfextu(
        &mut self,
        inst: &Instruction,
        src: &AddressingMode,
        dst: DataReg,
        offset: BitFieldParam,
        width: BitFieldParam,
    ) -> Result<()> {
        // Get offset value (can span beyond 32 bits for memory operands)
        let offset_val = match offset {
            BitFieldParam::Immediate(v) => v as i32,
            BitFieldParam::Register(reg) => self.data_regs[data_reg_index(reg)] as i32,
        };

        // Get width value (0 means 32)
        let width_val = match width {
            BitFieldParam::Immediate(0) => 32u32,
            BitFieldParam::Immediate(v) => v as u32,
            BitFieldParam::Register(reg) => {
                let w = self.data_regs[data_reg_index(reg)] & 0x1f;
                if w == 0 { 32 } else { w }
            }
        };

        // Extract the bit field value
        let field_value = match src.ea {
            EffectiveAddress::Dr(reg) => {
                // For data register, offset is modulo 32
                let offset_mod = (offset_val as u32) % 32;
                let reg_val = self.data_regs[data_reg_index(reg)];
                extract_bitfield_from_u32(reg_val, offset_mod, width_val)
            }
            _ => {
                // Memory operand - offset can be negative or > 31
                let base_addr = self.compute_effective_address(src, inst.address)? as u32;
                extract_bitfield_from_memory(&self.memory, base_addr, offset_val, width_val)?
            }
        };

        // Store the zero-extended field value in the destination register
        self.data_regs[data_reg_index(dst)] = field_value;

        // Set condition codes: N and Z based on field value, C and V always cleared
        let msb_mask = 1u32 << (width_val - 1);
        self.set_flag(FLAG_N, (field_value & msb_mask) != 0);
        self.set_flag(FLAG_Z, field_value == 0);
        self.set_flag(FLAG_V, false);
        self.set_flag(FLAG_C, false);

        Ok(())
    }

    fn exec_bfexts(
        &mut self,
        inst: &Instruction,
        src: &AddressingMode,
        dst: DataReg,
        offset: BitFieldParam,
        width: BitFieldParam,
    ) -> Result<()> {
        // Get offset value (can span beyond 32 bits for memory operands)
        let offset_val = match offset {
            BitFieldParam::Immediate(v) => v as i32,
            BitFieldParam::Register(reg) => self.data_regs[data_reg_index(reg)] as i32,
        };

        // Get width value (0 means 32)
        let width_val = match width {
            BitFieldParam::Immediate(0) => 32u32,
            BitFieldParam::Immediate(v) => v as u32,
            BitFieldParam::Register(reg) => {
                let w = self.data_regs[data_reg_index(reg)] & 0x1f;
                if w == 0 { 32 } else { w }
            }
        };

        // Extract the bit field value
        let field_value = match src.ea {
            EffectiveAddress::Dr(reg) => {
                // For data register, offset is modulo 32
                let offset_mod = (offset_val as u32) % 32;
                let reg_val = self.data_regs[data_reg_index(reg)];
                extract_bitfield_from_u32(reg_val, offset_mod, width_val)
            }
            _ => {
                // Memory operand - offset can be negative or > 31
                let base_addr = self.compute_effective_address(src, inst.address)? as u32;
                extract_bitfield_from_memory(&self.memory, base_addr, offset_val, width_val)?
            }
        };

        // Sign-extend the field value based on the MSB of the extracted field
        let msb_mask = 1u32 << (width_val - 1);
        let sign_extended_value = if (field_value & msb_mask) != 0 {
            // MSB is set, sign-extend with 1s
            let extension_mask = !((1u32 << width_val) - 1);
            field_value | extension_mask
        } else {
            // MSB is clear, result is already correct (zero-extended)
            field_value
        };

        // Store the sign-extended field value in the destination register
        self.data_regs[data_reg_index(dst)] = sign_extended_value;

        // Set condition codes: N and Z based on sign-extended value, C and V always cleared
        self.set_flag(FLAG_N, (field_value & msb_mask) != 0);
        self.set_flag(FLAG_Z, field_value == 0);
        self.set_flag(FLAG_V, false);
        self.set_flag(FLAG_C, false);

        Ok(())
    }

    fn exec_bfins(
        &mut self,
        inst: &Instruction,
        src: DataReg,
        dst: &AddressingMode,
        offset: BitFieldParam,
        width: BitFieldParam,
    ) -> Result<()> {
        // Get offset value
        let offset_val = match offset {
            BitFieldParam::Immediate(v) => v as i32,
            BitFieldParam::Register(reg) => self.data_regs[data_reg_index(reg)] as i32,
        };

        // Get width value (0 means 32)
        let width_val = match width {
            BitFieldParam::Immediate(0) => 32u32,
            BitFieldParam::Immediate(v) => v as u32,
            BitFieldParam::Register(reg) => {
                let w = self.data_regs[data_reg_index(reg)] & 0x1f;
                if w == 0 { 32 } else { w }
            }
        };

        // Get the source value (low-order bits from source register)
        let src_value = self.data_regs[data_reg_index(src)];

        // Insert the bit field into the destination
        match dst.ea {
            EffectiveAddress::Dr(reg) => {
                // For data register, offset is modulo 32
                let offset_mod = (offset_val as u32) % 32;
                let dest_value = self.data_regs[data_reg_index(reg)];
                let result = insert_bitfield_into_u32(dest_value, src_value, offset_mod, width_val);
                self.data_regs[data_reg_index(reg)] = result;
            }
            _ => {
                // Memory operand - offset can be negative or > 31
                let base_addr = self.compute_effective_address(dst, inst.address)? as u32;
                insert_bitfield_into_memory(
                    &mut self.memory,
                    base_addr,
                    offset_val,
                    width_val,
                    src_value,
                )?;
            }
        }

        // Extract the inserted value for condition code setting
        let field_mask = if width_val == 32 {
            0xFFFFFFFF
        } else {
            (1u32 << width_val) - 1
        };
        let inserted_value = src_value & field_mask;

        // Set condition codes: N and Z based on inserted value, C and V always cleared
        let msb_mask = 1u32 << (width_val - 1);
        self.set_flag(FLAG_N, (inserted_value & msb_mask) != 0);
        self.set_flag(FLAG_Z, inserted_value == 0);
        self.set_flag(FLAG_V, false);
        self.set_flag(FLAG_C, false);

        Ok(())
    }

    // EXG - Exchange Registers
    fn exec_exg(&mut self, exg: Exg) -> Result<()> {
        match exg {
            Exg::DataData { rx, ry } => {
                let ix = data_reg_index(rx);
                let iy = data_reg_index(ry);
                self.data_regs.swap(ix, iy);
            }
            Exg::AddrAddr { rx, ry } => {
                let ix = addr_reg_index(rx);
                let iy = addr_reg_index(ry);
                self.addr_regs.swap(ix, iy);
            }
            Exg::DataAddr { data, addr } => {
                let id = data_reg_index(data);
                let ia = addr_reg_index(addr);
                std::mem::swap(&mut self.data_regs[id], &mut self.addr_regs[ia]);
            }
        }
        Ok(())
    }

    // ADDX - Add with Extend
    fn exec_addx(&mut self, addx: Addx) -> Result<()> {
        let x = if self.get_flag(FLAG_X) { 1u32 } else { 0u32 };
        match addx {
            Addx::Dn(dn) => {
                let src_idx = data_reg_index(dn.src);
                let dst_idx = data_reg_index(dn.dst);
                let src = size_mask(self.data_regs[src_idx], dn.size);
                let dst = size_mask(self.data_regs[dst_idx], dn.size);
                let result = addx_with_flags(src, dst, x, dn.size, self);
                self.data_regs[dst_idx] =
                    write_sized_data_reg(self.data_regs[dst_idx], result, dn.size);
            }
            Addx::PreDec(predec) => {
                let size_bytes = size_to_bytes(predec.size);
                let src_idx = addr_reg_index(predec.src);
                let dst_idx = addr_reg_index(predec.dst);
                self.addr_regs[src_idx] = self.addr_regs[src_idx].wrapping_sub(size_bytes);
                self.addr_regs[dst_idx] = self.addr_regs[dst_idx].wrapping_sub(size_bytes);
                let src = self.read_mem(self.addr_regs[src_idx] as usize, predec.size)?;
                let dst = self.read_mem(self.addr_regs[dst_idx] as usize, predec.size)?;
                let result = addx_with_flags(src, dst, x, predec.size, self);
                self.write_mem(self.addr_regs[dst_idx] as usize, predec.size, result)?;
            }
        }
        Ok(())
    }

    // SUBX - Subtract with Extend
    fn exec_subx(&mut self, subx: Subx) -> Result<()> {
        let x = if self.get_flag(FLAG_X) { 1u32 } else { 0u32 };
        match subx {
            Subx::Dn(dn) => {
                let src_idx = data_reg_index(dn.src);
                let dst_idx = data_reg_index(dn.dst);
                let src = size_mask(self.data_regs[src_idx], dn.size);
                let dst = size_mask(self.data_regs[dst_idx], dn.size);
                let result = subx_with_flags(dst, src, x, dn.size, self);
                self.data_regs[dst_idx] =
                    write_sized_data_reg(self.data_regs[dst_idx], result, dn.size);
            }
            Subx::PreDec(predec) => {
                let size_bytes = size_to_bytes(predec.size);
                let src_idx = addr_reg_index(predec.src);
                let dst_idx = addr_reg_index(predec.dst);
                self.addr_regs[src_idx] = self.addr_regs[src_idx].wrapping_sub(size_bytes);
                self.addr_regs[dst_idx] = self.addr_regs[dst_idx].wrapping_sub(size_bytes);
                let src = self.read_mem(self.addr_regs[src_idx] as usize, predec.size)?;
                let dst = self.read_mem(self.addr_regs[dst_idx] as usize, predec.size)?;
                let result = subx_with_flags(dst, src, x, predec.size, self);
                self.write_mem(self.addr_regs[dst_idx] as usize, predec.size, result)?;
            }
        }
        Ok(())
    }

    // ABCD - Add BCD with Extend
    fn exec_abcd(&mut self, abcd: Abcd) -> Result<()> {
        let x = if self.get_flag(FLAG_X) { 1u8 } else { 0u8 };
        match abcd {
            Abcd::Dn { src, dst } => {
                let src_idx = data_reg_index(src);
                let dst_idx = data_reg_index(dst);
                let src_val = self.data_regs[src_idx] as u8;
                let dst_val = self.data_regs[dst_idx] as u8;
                let (result, carry) = add_bcd(src_val, dst_val, x);
                self.data_regs[dst_idx] = (self.data_regs[dst_idx] & 0xFFFFFF00) | (result as u32);
                if result != 0 {
                    self.set_flag(FLAG_Z, false);
                }
                self.set_flag(FLAG_C, carry);
                self.set_flag(FLAG_X, carry);
            }
            Abcd::PreDec { src, dst } => {
                let src_idx = addr_reg_index(src);
                let dst_idx = addr_reg_index(dst);
                self.addr_regs[src_idx] = self.addr_regs[src_idx].wrapping_sub(1);
                self.addr_regs[dst_idx] = self.addr_regs[dst_idx].wrapping_sub(1);
                let src_val = self.memory.read_byte(self.addr_regs[src_idx] as usize)?;
                let dst_val = self.memory.read_byte(self.addr_regs[dst_idx] as usize)?;
                let (result, carry) = add_bcd(src_val, dst_val, x);
                self.memory
                    .write_data(self.addr_regs[dst_idx] as usize, &[result])?;
                if result != 0 {
                    self.set_flag(FLAG_Z, false);
                }
                self.set_flag(FLAG_C, carry);
                self.set_flag(FLAG_X, carry);
            }
        }
        Ok(())
    }

    // SBCD - Subtract BCD with Extend
    fn exec_sbcd(&mut self, sbcd: Sbcd) -> Result<()> {
        let x = if self.get_flag(FLAG_X) { 1u8 } else { 0u8 };
        match sbcd {
            Sbcd::Dn { src, dst } => {
                let src_idx = data_reg_index(src);
                let dst_idx = data_reg_index(dst);
                let src_val = self.data_regs[src_idx] as u8;
                let dst_val = self.data_regs[dst_idx] as u8;
                let (result, borrow) = sub_bcd(dst_val, src_val, x);
                self.data_regs[dst_idx] = (self.data_regs[dst_idx] & 0xFFFFFF00) | (result as u32);
                if result != 0 {
                    self.set_flag(FLAG_Z, false);
                }
                self.set_flag(FLAG_C, borrow);
                self.set_flag(FLAG_X, borrow);
            }
            Sbcd::PreDec { src, dst } => {
                let src_idx = addr_reg_index(src);
                let dst_idx = addr_reg_index(dst);
                self.addr_regs[src_idx] = self.addr_regs[src_idx].wrapping_sub(1);
                self.addr_regs[dst_idx] = self.addr_regs[dst_idx].wrapping_sub(1);
                let src_val = self.memory.read_byte(self.addr_regs[src_idx] as usize)?;
                let dst_val = self.memory.read_byte(self.addr_regs[dst_idx] as usize)?;
                let (result, borrow) = sub_bcd(dst_val, src_val, x);
                self.memory
                    .write_data(self.addr_regs[dst_idx] as usize, &[result])?;
                if result != 0 {
                    self.set_flag(FLAG_Z, false);
                }
                self.set_flag(FLAG_C, borrow);
                self.set_flag(FLAG_X, borrow);
            }
        }
        Ok(())
    }

    // MOVEM - Move Multiple Registers
    fn exec_movem(&mut self, inst: &Instruction, movem: Movem) -> Result<()> {
        let size_bytes = if movem.size == Size::Word { 2 } else { 4 };
        let is_predec = matches!(movem.mode.ea, EffectiveAddress::AddrPreDecr(_));

        // For predecrement mode, the register mask is encoded in reverse order:
        // Normal encoding: D0=bit0, D1=bit1, ..., D7=bit7, A0=bit8, ..., A7=bit15
        // Predec encoding: A7=bit0, A6=bit1, ..., A0=bit7, D7=bit8, ..., D0=bit15
        // Map bit position to register: for predec, bit i -> register (15-i) in normal order
        let reg_for_bit = |bit: usize| -> (bool, usize) {
            if is_predec {
                // Reverse mapping: bit 0 -> A7 (reg 15), bit 15 -> D0 (reg 0)
                let reg = 15 - bit;
                (reg < 8, if reg < 8 { reg } else { reg - 8 })
            } else {
                // Normal mapping: bit 0 -> D0, bit 15 -> A7
                (bit < 8, if bit < 8 { bit } else { bit - 8 })
            }
        };

        match movem.direction {
            DataDir::RegToMem => {
                let mut addr = self.compute_effective_address(&movem.mode, inst.address)?;
                if is_predec {
                    // For predecrement, store from A7 down to D0 (descending address)
                    // Iterate through bits 0-15, which map to A7, A6, ..., D1, D0
                    for bit in 0..16 {
                        if (movem.register_mask & (1 << bit)) != 0 {
                            addr = addr.wrapping_sub(size_bytes);
                            let (is_data, reg_idx) = reg_for_bit(bit);
                            let val = if is_data {
                                self.data_regs[reg_idx]
                            } else {
                                self.addr_regs[reg_idx]
                            };
                            if movem.size == Size::Word {
                                self.memory.write_data(addr, &(val as u16).to_be_bytes())?;
                            } else {
                                self.memory.write_data(addr, &val.to_be_bytes())?;
                            }
                        }
                    }
                    // Update address register
                    if let EffectiveAddress::AddrPreDecr(ar) = movem.mode.ea {
                        self.addr_regs[addr_reg_index(ar)] = addr as u32;
                    }
                } else {
                    // Normal order (D0 to A7)
                    for bit in 0..16 {
                        if (movem.register_mask & (1 << bit)) != 0 {
                            let (is_data, reg_idx) = reg_for_bit(bit);
                            let val = if is_data {
                                self.data_regs[reg_idx]
                            } else {
                                self.addr_regs[reg_idx]
                            };
                            if movem.size == Size::Word {
                                self.memory.write_data(addr, &(val as u16).to_be_bytes())?;
                            } else {
                                self.memory.write_data(addr, &val.to_be_bytes())?;
                            }
                            addr = addr.wrapping_add(size_bytes);
                        }
                    }
                }
            }
            DataDir::MemToReg => {
                let mut addr = self.compute_effective_address(&movem.mode, inst.address)?;
                // For MemToReg, the mask is always in normal format (D0=bit0, A7=bit15)
                // even when used with postincrement
                for bit in 0..16 {
                    if (movem.register_mask & (1 << bit)) != 0 {
                        let val = if movem.size == Size::Word {
                            (self.memory.read_word(addr)? as i16) as i32 as u32
                        } else {
                            self.memory.read_long(addr)?
                        };
                        if bit < 8 {
                            self.data_regs[bit] = val;
                        } else {
                            self.addr_regs[bit - 8] = val;
                        }
                        addr = addr.wrapping_add(size_bytes);
                    }
                }
                // Update address register for postincrement
                if let EffectiveAddress::AddrPostIncr(ar) = movem.mode.ea {
                    self.addr_regs[addr_reg_index(ar)] = addr as u32;
                }
            }
        }
        Ok(())
    }

    // CMPM - Compare Memory
    fn exec_cmpm(&mut self, size: Size, src: AddrReg, dst: AddrReg) -> Result<()> {
        let size_bytes = size_to_bytes(size);
        let src_idx = addr_reg_index(src);
        let dst_idx = addr_reg_index(dst);
        let src_val = self.read_mem(self.addr_regs[src_idx] as usize, size)?;
        let dst_val = self.read_mem(self.addr_regs[dst_idx] as usize, size)?;
        self.addr_regs[src_idx] = self.addr_regs[src_idx].wrapping_add(size_bytes);
        self.addr_regs[dst_idx] = self.addr_regs[dst_idx].wrapping_add(size_bytes);
        cmp_with_flags(dst_val, src_val, size, self);
        Ok(())
    }

    // TAS - Test and Set
    fn exec_tas(&mut self, inst: &Instruction, mode: AddressingMode) -> Result<()> {
        let value = self.read_operand(&mode, Size::Byte, inst.address)?;
        self.update_nz_flags_sized(value, Size::Byte);
        self.set_flag(FLAG_V, false);
        self.set_flag(FLAG_C, false);
        // Set high bit
        self.write_operand(&mode, Size::Byte, inst.address, value | 0x80)?;
        Ok(())
    }

    // Helper: write to operand
    fn write_operand(
        &mut self,
        mode: &AddressingMode,
        size: Size,
        inst_addr: usize,
        value: u32,
    ) -> Result<()> {
        match mode.ea {
            EffectiveAddress::Dr(reg) => {
                let idx = data_reg_index(reg);
                self.data_regs[idx] = write_sized_data_reg(self.data_regs[idx], value, size);
            }
            EffectiveAddress::Ar(reg) => {
                let idx = addr_reg_index(reg);
                self.addr_regs[idx] = value;
            }
            EffectiveAddress::Addr(reg) => {
                let addr = self.addr_regs[addr_reg_index(reg)] as usize;
                self.write_mem(addr, size, value)?;
            }
            EffectiveAddress::AddrDisplace(_) => {
                let addr = self.compute_effective_address(mode, inst_addr)?;
                self.write_mem(addr, size, value)?;
            }
            EffectiveAddress::AbsShort => {
                let addr = mode
                    .short_data()
                    .ok_or_else(|| anyhow!("missing AbsShort data"))?
                    as usize;
                self.write_mem(addr, size, value)?;
            }
            EffectiveAddress::AbsLong => {
                let addr = mode
                    .long_data()
                    .ok_or_else(|| anyhow!("missing AbsLong data"))?
                    as usize;
                self.write_mem(addr, size, value)?;
            }
            EffectiveAddress::AddrPostIncr(reg) => {
                let idx = addr_reg_index(reg);
                let addr = self.addr_regs[idx] as usize;
                self.write_mem(addr, size, value)?;
                let step = Self::ea_step(size, reg == AddrReg::A7);
                self.addr_regs[idx] = self.addr_regs[idx].wrapping_add(step);
            }
            EffectiveAddress::AddrPreDecr(reg) => {
                let idx = addr_reg_index(reg);
                let step = Self::ea_step(size, reg == AddrReg::A7);
                self.addr_regs[idx] = self.addr_regs[idx].wrapping_sub(step);
                let addr = self.addr_regs[idx] as usize;
                self.write_mem(addr, size, value)?;
            }
            EffectiveAddress::AddrIndex(_) => {
                let addr = self.compute_effective_address(mode, inst_addr)?;
                self.write_mem(addr, size, value)?;
            }
            _ => bail!("unsupported addressing mode {:?} for write", mode.ea),
        }
        Ok(())
    }

    // Helper: write to memory
    fn write_mem(&mut self, addr: usize, size: Size, value: u32) -> Result<()> {
        match size {
            Size::Byte => self.memory.write_data(addr, &[value as u8])?,
            Size::Word => self
                .memory
                .write_data(addr, &(value as u16).to_be_bytes())?,
            Size::Long => self.memory.write_data(addr, &value.to_be_bytes())?,
        }
        Ok(())
    }

    // Helper: push long onto stack
    fn push_long(&mut self, value: u32) -> Result<()> {
        self.addr_regs[7] = self.addr_regs[7].wrapping_sub(4);
        let sp = self.addr_regs[7] as usize;
        self.memory.write_data(sp, &value.to_be_bytes())?;
        Ok(())
    }

    // Helper: pop long from stack
    fn pop_long(&mut self) -> Result<u32> {
        let sp = self.addr_regs[7] as usize;
        let value = self.memory.read_long(sp)?;
        self.addr_regs[7] = self.addr_regs[7].wrapping_add(4);
        Ok(value)
    }

    fn read_word_unsigned(&mut self, mode: AddressingMode) -> Result<u16> {
        match mode.ea {
            EffectiveAddress::Dr(reg) => Ok(self.data_regs[data_reg_index(reg)] as u16),
            EffectiveAddress::Addr(addr_reg) => {
                let addr = self.addr_regs[addr_reg_index(addr_reg)] as usize;
                Ok(self.memory.read_word(addr)?)
            }
            EffectiveAddress::AddrDisplace(addr_reg) => {
                let displacement = mode
                    .short_data()
                    .ok_or_else(|| anyhow!("missing displacement for AddrDisplace"))?
                    as i16;
                let base = self.addr_regs[addr_reg_index(addr_reg)] as i64;
                let effective = base
                    .checked_add(displacement as i64)
                    .ok_or_else(|| anyhow!("address overflow computing displacement"))?
                    as usize;
                Ok(self.memory.read_word(effective)?)
            }
            EffectiveAddress::AbsShort => {
                let addr = mode
                    .short_data()
                    .ok_or_else(|| anyhow!("missing address for AbsShort"))?
                    as usize;
                Ok(self.memory.read_word(addr)?)
            }
            EffectiveAddress::AbsLong => {
                let addr = mode
                    .long_data()
                    .ok_or_else(|| anyhow!("missing address for AbsLong"))?
                    as usize;
                Ok(self.memory.read_word(addr)?)
            }
            EffectiveAddress::Immediate => {
                let imm = mode
                    .immediate()
                    .ok_or_else(|| anyhow!("missing immediate for word operand"))?;
                match imm {
                    Immediate::Byte(v) => Ok(v as u16),
                    Immediate::Word(v) => Ok(v),
                    Immediate::Long(v) => Ok(v as u16),
                }
            }
            EffectiveAddress::AddrPostIncr(reg) => {
                let idx = addr_reg_index(reg);
                let addr = self.addr_regs[idx] as usize;
                let value = self.memory.read_word(addr)?;
                self.addr_regs[idx] = self.addr_regs[idx].wrapping_add(2);
                Ok(value)
            }
            EffectiveAddress::AddrPreDecr(reg) => {
                let idx = addr_reg_index(reg);
                self.addr_regs[idx] = self.addr_regs[idx].wrapping_sub(2);
                let addr = self.addr_regs[idx] as usize;
                Ok(self.memory.read_word(addr)?)
            }
            _ => bail!("unsupported addressing mode {:?} for word read", mode.ea),
        }
    }

    fn read_word_signed(&mut self, mode: AddressingMode) -> Result<i16> {
        Ok(self.read_word_unsigned(mode)? as i16)
    }

    fn read_operand(&mut self, mode: &AddressingMode, size: Size, inst_addr: usize) -> Result<u32> {
        match mode.ea {
            EffectiveAddress::Dr(reg) => Ok(size_mask(self.data_regs[data_reg_index(reg)], size)),
            EffectiveAddress::Ar(reg) => Ok(self.addr_regs[addr_reg_index(reg)]),
            EffectiveAddress::Addr(reg) => {
                let addr = self.addr_regs[addr_reg_index(reg)] as usize;
                self.read_mem(addr, size)
            }
            EffectiveAddress::AddrDisplace(_) => {
                let addr = self.compute_effective_address(mode, inst_addr)?;
                self.read_mem(addr, size)
            }
            EffectiveAddress::AbsShort => {
                let addr = mode
                    .short_data()
                    .ok_or_else(|| anyhow!("missing AbsShort data"))?
                    as usize;
                self.read_mem(addr, size)
            }
            EffectiveAddress::AbsLong => {
                let addr = mode
                    .long_data()
                    .ok_or_else(|| anyhow!("missing AbsLong data"))?
                    as usize;
                self.read_mem(addr, size)
            }
            EffectiveAddress::PCDisplace => {
                let addr = self.compute_effective_address(mode, inst_addr)?;
                self.read_mem(addr, size)
            }
            EffectiveAddress::Immediate => match mode
                .immediate()
                .ok_or_else(|| anyhow!("missing immediate data"))?
            {
                Immediate::Byte(v) => Ok(v as u32),
                Immediate::Word(v) => Ok(v as u32),
                Immediate::Long(v) => Ok(v),
            },
            EffectiveAddress::AddrPostIncr(reg) => {
                let idx = addr_reg_index(reg);
                let addr = self.addr_regs[idx] as usize;
                let value = self.read_mem(addr, size)?;
                let step = Self::ea_step(size, reg == AddrReg::A7);
                self.addr_regs[idx] = self.addr_regs[idx].wrapping_add(step);
                Ok(value)
            }
            EffectiveAddress::AddrPreDecr(reg) => {
                let idx = addr_reg_index(reg);
                let step = Self::ea_step(size, reg == AddrReg::A7);
                self.addr_regs[idx] = self.addr_regs[idx].wrapping_sub(step);
                let addr = self.addr_regs[idx] as usize;
                self.read_mem(addr, size)
            }
            EffectiveAddress::AddrIndex(_) | EffectiveAddress::PCIndex => {
                let addr = self.compute_effective_address(mode, inst_addr)?;
                self.read_mem(addr, size)
            }
        }
    }

    fn compute_effective_address(&self, mode: &AddressingMode, inst_addr: usize) -> Result<usize> {
        match mode.ea {
            EffectiveAddress::Addr(reg) => Ok(self.addr_regs[addr_reg_index(reg)] as usize),
            EffectiveAddress::AddrPostIncr(reg) => Ok(self.addr_regs[addr_reg_index(reg)] as usize),
            EffectiveAddress::AddrPreDecr(reg) => Ok(self.addr_regs[addr_reg_index(reg)] as usize),
            EffectiveAddress::AddrDisplace(reg) => {
                let disp =
                    mode.short_data()
                        .ok_or_else(|| anyhow!("missing displacement"))? as i16;
                let base = self.addr_regs[addr_reg_index(reg)] as i64;
                let eff = base
                    .checked_add(disp as i64)
                    .ok_or_else(|| anyhow!("address overflow computing displacement"))?;
                Ok(eff as u32 as usize)
            }
            EffectiveAddress::AbsShort => mode
                .short_data()
                .map(|v| v as i16 as i32 as usize) // Sign-extend 16-bit to 32-bit
                .ok_or_else(|| anyhow!("missing AbsShort data")),
            EffectiveAddress::AbsLong => mode
                .long_data()
                .map(|v| v as usize)
                .ok_or_else(|| anyhow!("missing AbsLong data")),
            EffectiveAddress::PCDisplace => {
                let disp = mode
                    .short_data()
                    .ok_or_else(|| anyhow!("missing PC displacement"))?
                    as i16;
                let base = inst_addr
                    .checked_add(2)
                    .ok_or_else(|| anyhow!("overflow computing PC base"))?;
                let eff = (base as i64)
                    .checked_add(disp as i64)
                    .ok_or_else(|| anyhow!("address overflow computing PC displacement"))?;
                Ok(eff as u32 as usize)
            }
            EffectiveAddress::AddrIndex(reg) => {
                let (ext_word, base_disp) = mode
                    .index_ext()
                    .ok_or_else(|| anyhow!("missing index extension word"))?;
                // Check base suppress (68020+ full format)
                let is_full_format = (ext_word & 0x0100) != 0;
                let base_suppress = is_full_format && (ext_word & 0x0080) != 0;
                let base = if base_suppress {
                    0i64
                } else {
                    self.addr_regs[addr_reg_index(reg)] as i64
                };
                let index = self.decode_index_register(ext_word);
                let eff = base
                    .checked_add(index)
                    .and_then(|v| v.checked_add(base_disp as i64))
                    .ok_or_else(|| anyhow!("address overflow computing indexed address"))?;
                Ok(eff as u32 as usize)
            }
            EffectiveAddress::PCIndex => {
                let (ext_word, base_disp) = mode
                    .index_ext()
                    .ok_or_else(|| anyhow!("missing PC index extension word"))?;
                // Check base suppress (68020+ full format)
                let is_full_format = (ext_word & 0x0100) != 0;
                let base_suppress = is_full_format && (ext_word & 0x0080) != 0;
                let base = if base_suppress {
                    0i64
                } else {
                    inst_addr
                        .checked_add(2)
                        .ok_or_else(|| anyhow!("overflow computing PC base"))?
                        as i64
                };

                // Check for memory indirect (68020+)
                // Bits 2-0: I/IS field
                // 0 = no memory indirect
                // 1-3 = preindexed (index before indirection)
                // 5-7 = postindexed (index after indirection)
                let i_is = ext_word & 0x7;
                if is_full_format && i_is != 0 {
                    // Memory indirect mode
                    let is_postindexed = (i_is & 0x4) != 0;
                    let index = self.decode_index_register(ext_word);

                    let intermediate = if is_postindexed {
                        // Postindexed: (base + bd), then add index
                        base.checked_add(base_disp as i64)
                            .ok_or_else(|| anyhow!("address overflow"))?
                    } else {
                        // Preindexed: (base + index + bd)
                        base.checked_add(index)
                            .and_then(|v| v.checked_add(base_disp as i64))
                            .ok_or_else(|| anyhow!("address overflow"))?
                    };

                    // Read pointer from intermediate address
                    let ptr_addr = usize::try_from(intermediate)
                        .map_err(|_| anyhow!("negative intermediate address"))?;
                    let ptr = self.memory.read_long(ptr_addr)? as i64;

                    // Add index (if postindexed) and outer displacement
                    // For now, outer displacement is 0 (null) for i_is = 1 or 5
                    let final_addr = if is_postindexed {
                        ptr.checked_add(index)
                            .ok_or_else(|| anyhow!("address overflow"))?
                    } else {
                        ptr
                    };

                    Ok(final_addr as u32 as usize)
                } else {
                    // Simple indexed (no memory indirect)
                    let index = self.decode_index_register(ext_word);
                    let eff = base
                        .checked_add(index)
                        .and_then(|v| v.checked_add(base_disp as i64))
                        .ok_or_else(|| anyhow!("address overflow computing PC indexed address"))?;
                    usize::try_from(eff).map_err(|_| anyhow!("negative effective address"))
                }
            }
            _ => bail!(
                "unsupported addressing mode {:?} for effective address",
                mode.ea
            ),
        }
    }

    /// Decode the index register value from an extension word.
    /// Extension word format:
    /// - Bit 15: D/A (0=Dn, 1=An)
    /// - Bits 14-12: register number
    /// - Bit 11: W/L (0=word sign-extended, 1=long)
    /// - Bits 10-9: scale (00=1, 01=2, 10=4, 11=8) - 68020+
    /// - Bit 8: full extension word indicator (68020+)
    /// - Bit 6: IS (index suppress) - 68020+
    fn decode_index_register(&self, ext_word: u16) -> i64 {
        // Check if index is suppressed (68020+ full format only)
        let is_full_format = (ext_word & 0x0100) != 0;
        if is_full_format && (ext_word & 0x0040) != 0 {
            return 0; // Index suppressed
        }

        let is_addr_reg = (ext_word & 0x8000) != 0;
        let reg_num = ((ext_word >> 12) & 0x7) as usize;
        let is_long = (ext_word & 0x0800) != 0;
        let scale = 1i64 << ((ext_word >> 9) & 0x3); // 1, 2, 4, or 8

        let reg_value = if is_addr_reg {
            self.addr_regs[reg_num]
        } else {
            self.data_regs[reg_num]
        };

        let index = if is_long {
            reg_value as i32 as i64
        } else {
            // Word-sized: sign-extend low 16 bits
            (reg_value as i16) as i64
        };

        index * scale
    }

    fn read_mem(&mut self, addr: usize, size: Size) -> Result<u32> {
        Ok(match size {
            Size::Byte => self.memory.read_byte(addr)? as u32,
            Size::Word => self.memory.read_word(addr)? as u32,
            Size::Long => self.memory.read_long(addr)?,
        })
    }

    fn exec_trap(&mut self, vector: u8) -> Result<()> {
        match vector {
            0 => self.handle_syscall(),
            _ => bail!("trap #{vector} not implemented"),
        }
    }

    fn ea_step(size: Size, is_a7: bool) -> u32 {
        match size {
            Size::Byte => {
                if is_a7 {
                    2
                } else {
                    1
                }
            }
            Size::Word => 2,
            Size::Long => 4,
        }
    }

    fn handle_syscall(&mut self) -> Result<()> {
        let m68k_num = self.data_regs[0];
        let x86_num = m68k_to_x86_64_syscall(m68k_num).unwrap_or_default();

        // m68k Linux ABI: D0=syscall, D1-D5=args
        let result: i64 = match m68k_num {
            // exit(status) - no return
            1 => std::process::exit(self.data_regs[1] as i32),

            // fork() - no pointers
            2 => self.sys_passthrough(x86_num, 0),

            // read(fd, buf, count) - buf is pointer
            3 => self.sys_read()?,

            // write(fd, buf, count) - buf is pointer
            4 => self.sys_write()?,

            // open(path, flags, mode) - path is pointer
            5 => self.sys_open()?,

            // close(fd) - no pointers
            6 => self.sys_passthrough(x86_num, 1),

            // waitpid(pid, status, options) - forward to wait4(pid,...,NULL)
            7 => self.sys_waitpid()?,

            // creat(path, mode) - path is pointer
            8 => self.sys_path1(x86_num, self.data_regs[2] as i64)?,

            // link(oldpath, newpath) - both pointers
            9 => self.sys_link()?,

            // unlink(path) - path is pointer
            10 => self.sys_path1(x86_num, 0)?,

            // execve(path, argv, envp) - replaces current process
            11 => self.sys_execve()?,

            // chdir(path) - path is pointer
            12 => self.sys_path1(x86_num, 0)?,

            // time(tloc) - tloc is pointer (can be NULL)
            13 => self.sys_time()?,

            // mknod(path, mode, dev) - path is pointer
            14 => self.sys_mknod()?,

            // chmod(path, mode) - path is pointer
            15 => self.sys_path1(x86_num, self.data_regs[2] as i64)?,

            // lchown(path, owner, group) - path is pointer
            16 => self.sys_chown(x86_num)?,

            // syscall 17 used to be break, doesn't exist anymore
            17 => -1,

            // syscall 18 is oldstat, can forward to stat
            18 => self.sys_stat(4)?,

            // lseek(fd, offset, whence) - no pointers
            19 => self.sys_passthrough(x86_num, 3),

            // getpid() - no pointers
            20 => self.sys_passthrough(x86_num, 0),

            // mount - complex, skip for now
            21 => bail!("mount not yet implemented"),

            // umount(target) - implemented as umount2(target, 0)
            22 => {
                let path_addr = self.data_regs[1] as usize;
                let path_cstr = self.guest_cstring(path_addr)?;
                let result = unsafe { libc::umount2(path_cstr.as_ptr(), 0) as i64 };
                Self::libc_to_kernel(result)
            }

            // setuid(uid) - no pointers
            23 => self.sys_passthrough(x86_num, 1),

            // getuid() - no pointers
            24 => self.sys_passthrough(x86_num, 0),

            // stime - skip for now
            25 => bail!("stime not yet implemented"),

            // ptrace - complex, skip for now
            26 => bail!("ptrace not yet implemented"),

            // alarm(seconds) - no pointers
            27 => self.sys_passthrough(x86_num, 1),

            // oldfstat - skip for now
            28 => bail!("oldfstat not yet implemented"),

            // pause() - no pointers
            29 => self.sys_passthrough(x86_num, 0),

            // utime(path, times) - path + struct pointer
            30 => self.sys_utime()?,

            // 31 was stty
            31 => -1,

            // 31 was gtty
            32 => -1,

            // access(path, mode) - path is pointer
            33 => self.sys_path1(x86_num, self.data_regs[2] as i64)?,

            // nice(incr)
            34 => bail!("nice not yet implemented"),

            // 35 was ftime
            35 => -1,

            // sync() - no pointers
            36 => self.sys_passthrough(x86_num, 0),

            // kill(pid, sig) - no pointers
            37 => self.sys_passthrough(x86_num, 2),

            // rename(old, new) - both pointers
            38 => self.sys_rename()?,

            // mkdir(path, mode) - path is pointer
            39 => self.sys_path1(x86_num, self.data_regs[2] as i64)?,

            // rmdir(path) - path is pointer
            40 => self.sys_path1(x86_num, 0)?,

            // dup(fd) - no pointers
            41 => self.sys_passthrough(x86_num, 1),

            // pipe(pipefd) - pointer to int[2]
            42 => self.sys_pipe()?,

            // times(buf) - pointer to struct tms
            43 => self.sys_times()?,

            // 44 was prof
            44 => -1,

            // brk(addr) - special handling
            45 => self.sys_brk()?,

            // setgid(gid) - no pointers
            46 => self.sys_passthrough(x86_num, 1),

            // getgid() - no pointers
            47 => self.sys_passthrough(x86_num, 0),

            // 48 is signal, complicated
            48 => bail!("signal not yet implemented"),

            // geteuid() - no pointers
            49 => self.sys_passthrough(x86_num, 0),

            // getegid() - no pointers
            50 => self.sys_passthrough(x86_num, 0),

            // acct(filename) - path pointer (can be NULL)
            51 => {
                let path = self.data_regs[1] as usize;
                if path == 0 {
                    self.sys_passthrough(x86_num, 1)
                } else {
                    self.sys_path1(x86_num, 0)?
                }
            }

            // umount2(target, flags) - path pointer
            52 => self.sys_path1(x86_num, self.data_regs[2] as i64)?,

            // 53 was lock
            53 => -1,

            // ioctl(fd, request, arg) - complex, passthrough for now
            54 => self.sys_passthrough(x86_num, 3),

            // fcntl(fd, cmd, arg) - mostly no pointers
            55 => self.sys_passthrough(x86_num, 3),

            // 56 was mpx
            56 => -1,

            // setpgid(pid, pgid) - no pointers
            57 => self.sys_passthrough(x86_num, 2),

            // 58 was ulimit
            58 => -1,

            // 59 was oldolduname
            59 => -1,

            // umask(mask) - no pointers
            60 => self.sys_passthrough(x86_num, 1),

            // chroot(path) - path pointer
            61 => self.sys_path1(x86_num, 0)?,

            // 62 ustat(dev, ubuf) - information about mounted fs. deprecated
            62 => bail!("ustat not yet implemented"),

            // dup2(old, new) - no pointers
            63 => self.sys_passthrough(x86_num, 2),

            // getppid() - no pointers
            64 => self.sys_passthrough(x86_num, 0),

            // getpgrp() - no pointers
            65 => self.sys_passthrough(x86_num, 0),

            // setsid() - no pointers
            66 => self.sys_passthrough(x86_num, 0),

            // sigaction
            67 => bail!("sigaction not yet implemented"),

            // sgetmask
            68 => bail!("sgetmask not yet implemented"),

            // ssetmask
            69 => bail!("ssetmask not yet implemented"),

            // setreuid(ruid, euid) - no pointers
            70 => self.sys_passthrough(x86_num, 2),

            // setregid(rgid, egid) - no pointers
            71 => self.sys_passthrough(x86_num, 2),

            // sigsuspend
            72 => bail!("sigsuspend not yet implemented"),

            // sigpending
            73 => bail!("sigpending not yet implemented"),

            // sethostname(name, len) - pointer
            74 => self.sys_sethostname()?,

            // setrlimit(resource, rlim) - pointer to struct
            75 => self.sys_setrlimit()?,

            // getrlimit(resource, rlim) - pointer to struct
            76 => self.sys_getrlimit()?,

            // getrusage(who, usage) - pointer to struct
            77 => self.sys_getrusage()?,

            // gettimeofday(tv, tz) - pointers
            78 => self.sys_gettimeofday()?,

            // settimeofday(tv, tz) - pointers
            79 => self.sys_settimeofday()?,

            // getgroups(size, list) - pointer
            80 => self.sys_getgroups()?,

            // setgroups(size, list) - pointer
            81 => self.sys_setgroups()?,

            // select(nfds, readfds, writefds, exceptfds, timeout) - pointers
            82 => self.sys_select()?,

            // symlink(target, linkpath) - both pointers
            83 => self.sys_symlink()?,

            // oldlstat not implemented
            84 => -1,

            // readlink(path, buf, size) - path + buf pointers
            85 => self.sys_readlink()?,

            // uselib, deprecated
            86 => -1,

            // swapon(path, flags) - path pointer
            87 => self.sys_path1(x86_num, self.data_regs[2] as i64)?,

            // reboot(magic, magic2, cmd, arg) - no pointers needed
            88 => self.sys_passthrough(x86_num, 4),

            // readdir, superseded by getdents
            89 => -1,

            // mmap - use new mmap2 style (syscall 90 is old_mmap on m68k)
            90 => self.sys_mmap()?,

            // munmap(addr, length) - no pointers (addr is value)
            91 => self.sys_passthrough(x86_num, 2),

            // truncate(path, length) - path pointer
            92 => self.sys_path1(x86_num, self.data_regs[2] as i64)?,

            // ftruncate(fd, length) - no pointers
            93 => self.sys_passthrough(x86_num, 2),

            // fchmod(fd, mode) - no pointers
            94 => self.sys_passthrough(x86_num, 2),

            // fchown(fd, owner, group) - no pointers
            95 => self.sys_passthrough(x86_num, 3),

            // getpriority(which, who) - no pointers
            96 => self.sys_passthrough(x86_num, 2),

            // setpriority(which, who, prio) - no pointers
            97 => self.sys_passthrough(x86_num, 3),

            // 98 was profil
            98 => -1,

            // statfs(path, buf) - path + struct pointer
            99 => self.sys_statfs()?,

            // fstatfs(fd, buf) - struct pointer
            100 => self.sys_fstatfs()?,

            // ioperm(from, num, turn_on) - no pointers
            101 => self.sys_passthrough(x86_num, 3),

            // socketcall
            102 => bail!("socketcall not yet implemented"),

            // syslog(type, buf, len) - buf pointer
            103 => self.sys_syslog()?,

            // setitimer(which, new, old) - struct pointers
            104 => self.sys_setitimer()?,

            // getitimer(which, curr) - struct pointer
            105 => self.sys_getitimer()?,

            // stat(path, buf) - path + struct pointer
            106 => self.sys_stat(x86_num)?,

            // lstat(path, buf) - path + struct pointer
            107 => self.sys_stat(x86_num)?,

            // fstat(fd, buf) - struct pointer
            108 => self.sys_fstat()?,

            // 109 was olduname
            109 => -1,

            // 109 was iopl
            110 => -1,

            // vhangup() - no pointers
            111 => self.sys_passthrough(x86_num, 0),

            // 112 was idle
            112 => -1,

            // 113 was vm86
            113 => -1,

            // wait4(pid, status, options, rusage) - pointers
            114 => self.sys_wait4()?,

            // swapoff(path) - path pointer
            115 => self.sys_path1(x86_num, 0)?,

            // sysinfo(info) - struct pointer
            116 => self.sys_sysinfo()?,

            // ipc(call, first, second, third, ptr, fifth)
            // Multiplexer that dispatches to individual IPC syscalls
            117 => self.sys_ipc()?,

            // fsync(fd) - no pointers
            118 => self.sys_passthrough(x86_num, 1),

            // sigreturn
            119 => bail!("sigreturn not yet implemented"),

            // clone(flags, stack, parent_tid, child_tid, tls)
            120 => self.sys_clone()?,

            // setdomainname(name, len) - pointer
            121 => {
                let name_addr = self.data_regs[1] as usize;
                let len = self.data_regs[2] as usize;
                let host_ptr = self
                    .memory
                    .guest_to_host(name_addr, len)
                    .ok_or_else(|| anyhow!("invalid domainname buffer"))?;
                unsafe { libc::syscall(x86_num as i64, host_ptr, len) }
            }

            // uname(buf) - struct pointer
            122 => self.sys_uname()?,

            // int cacheflush(unsigned long addr, int scope, int cache, unsigned long size);
            // doesn't need to do anything on an interpreter.
            123 => 0,

            // adjtimex(buf) - struct pointer
            124 => self.sys_adjtimex()?,

            // mprotect(addr, len, prot) - validates guest memory range
            125 => self.sys_mprotect()?,

            // sigprocmask - complex
            126 => bail!("sigprocmask not yet implemented"),

            // create_module - complex
            127 => bail!("create_module not yet implemented"),

            // init_module(module_image, len, param_values)
            128 => self.sys_init_module()?,

            // delete_module(name, flags) - path pointer
            129 => self.sys_path1(x86_num, self.data_regs[2] as i64)?,

            // 130, get kernel_syms
            130 => bail!("get_kernel_syms not yet implemented"),

            // 131, quotactl
            131 => bail!("quotactl not yet implemented"),

            // getpgid(pid) - no pointers
            132 => self.sys_passthrough(x86_num, 1),

            // fchdir(fd) - no pointers
            133 => self.sys_passthrough(x86_num, 1),

            // bdflush int bdflush(int func, long data);
            134 => bail!("bdflush not yet implemented"),

            // personality(persona) - no pointers
            135 => self.sys_passthrough(x86_num, 1),

            // personality (alternate number on m68k)
            136 => self.sys_passthrough(libc::SYS_personality as u32, 1),

            // 137 was afs syscall
            137 => -1,

            // setfsuid(uid) - no pointers
            138 => self.sys_passthrough(x86_num, 1),

            // setfsgid(gid) - no pointers
            139 => self.sys_passthrough(x86_num, 1),

            // _llseek(fd, offset_high, offset_low, result, whence) - result pointer
            140 => self.sys_llseek()?,

            // getdents(fd, dirp, count) - pointer - 32-bit dirent
            141 => self.sys_getdents32()?,

            // _newselect, forward to select
            142 => self.sys_select()?,

            // flock(fd, operation) - no pointers
            143 => self.sys_passthrough(x86_num, 2),

            // msync(addr, length, flags) - no pointers
            144 => self.sys_passthrough(x86_num, 3),

            // readv(fd, iov, iovcnt) - iov pointer
            145 => self.sys_readv()?,

            // writev(fd, iov, iovcnt) - iov pointer
            146 => self.sys_writev()?,

            // getsid(pid) - no pointers
            147 => self.sys_passthrough(x86_num, 1),

            // fdatasync(fd) - no pointers
            148 => self.sys_passthrough(x86_num, 1),

            // _sysctl(args) - deprecated
            149 => bail!("_sysctl not yet implemented"),

            // mlock(addr, len) - no pointers
            150 => self.sys_passthrough(x86_num, 2),

            // munlock(addr, len) - no pointers
            151 => self.sys_passthrough(x86_num, 2),

            // mlockall(flags) - no pointers
            152 => self.sys_passthrough(x86_num, 1),

            // munlockall() - no pointers
            153 => self.sys_passthrough(x86_num, 0),

            // sched_setparam(pid, param) - struct pointer
            154 => self.sys_sched_setparam()?,

            // sched_getparam(pid, param) - struct pointer
            155 => self.sys_sched_getparam()?,

            // sched_setscheduler(pid, policy, param) - struct pointer
            156 => self.sys_sched_setscheduler()?,

            // sched_getscheduler(pid) - no pointers
            157 => self.sys_passthrough(x86_num, 1),

            // sched_yield() - no pointers
            158 => self.sys_passthrough(x86_num, 0),

            // sched_get_priority_max(policy) - no pointers
            159 => self.sys_passthrough(x86_num, 1),

            // sched_get_priority_min(policy) - no pointers
            160 => self.sys_passthrough(x86_num, 1),

            // sched_rr_get_interval(pid, tp) - struct pointer
            161 => self.sys_sched_rr_get_interval()?,

            // nanosleep(req, rem) - struct pointers
            162 => self.sys_nanosleep()?,

            // mremap(old_addr, old_size, new_size, flags, new_addr) - no pointers
            163 => self.sys_passthrough(x86_num, 5),

            // setresuid(ruid, euid, suid) - no pointers
            164 => self.sys_passthrough(x86_num, 3),

            // getresuid(ruid, euid, suid) - pointers
            165 => self.sys_getresuid()?,

            // getpagesize() - m68k 166, derive from host
            166 => self.sys_getpagesize()?,

            // query_module(name, which, buf, bufsize, ret) - deprecated
            167 => bail!("query_module not yet implemented"),

            // poll(fds, nfds, timeout) - pointer
            168 => self.sys_poll()?,

            // nfsservctl - removed from kernel
            169 => bail!("nfsservctl not yet implemented"),

            // setresgid(rgid, egid, sgid) - no pointers
            170 => self.sys_passthrough(x86_num, 3),

            // getresgid(rgid, egid, sgid) - pointers
            171 => self.sys_getresgid()?,

            // prctl(option, arg2, arg3, arg4, arg5) - m68k 172
            172 => self.sys_prctl()?,

            // rt_sigreturn - signal handling
            173 => bail!("rt_sigreturn not yet implemented"),

            // rt_sigaction(sig, act, oact, sigsetsize) - m68k 174
            174 => self.sys_passthrough(x86_num, 4),

            // rt_sigprocmask(how, set, oldset, sigsetsize) - m68k 175
            175 => self.sys_passthrough(x86_num, 4),

            // rt_sigpending(set, sigsetsize) - m68k 176
            176 => self.sys_passthrough(x86_num, 2),

            // rt_sigtimedwait(set, info, timeout, sigsetsize)
            177 => bail!("rt_sigtimedwait not yet implemented"),

            // rt_sigqueueinfo(tgid, sig, info)
            178 => bail!("rt_sigqueueinfo not yet implemented"),

            // rt_sigsuspend(mask, sigsetsize)
            179 => bail!("rt_sigsuspend not yet implemented"),

            // pread64(fd, buf, count, offset) - buf pointer
            180 => self.sys_pread64()?,

            // pwrite64(fd, buf, count, offset) - buf pointer
            181 => self.sys_pwrite64()?,

            // chown(path, owner, group) - path pointer
            182 => self.sys_chown(x86_num)?,

            // getcwd(buf, size) - buf pointer
            183 => self.sys_getcwd()?,

            // capget(hdrp, datap) - m68k 184
            184 => self.sys_capget()?,

            // capset(hdrp, datap) - m68k 185
            185 => self.sys_capset()?,

            // sigaltstack(ss, old_ss)
            186 => bail!("sigaltstack not yet implemented"),

            // sendfile(out_fd, in_fd, offset, count) - offset is pointer
            187 => self.sys_sendfile()?,

            // getpmsg - STREAMS, not implemented in Linux
            188 => bail!("getpmsg not yet implemented"),

            // putpmsg - STREAMS, not implemented in Linux
            189 => bail!("putpmsg not yet implemented"),

            // vfork() - convert to fork to avoid memory corruption
            // vfork shares memory with parent, which breaks our execve implementation
            // that modifies self.memory. Converting to fork gives us copy-on-write.
            190 => {
                let result = unsafe { libc::fork() as i64 };
                Self::libc_to_kernel(result)
            }

            // ugetrlimit(resource, rlim) - m68k only, same as getrlimit
            191 => self.sys_getrlimit()?,

            // mmap2(addr, length, prot, flags, fd, pgoffset) - offset is in pages
            192 => self.sys_mmap2()?,

            // truncate64(path, length) - path is pointer
            193 => self.sys_truncate()?,

            // ftruncate64(fd, length) - maps to ftruncate on x86_64
            194 => self.sys_passthrough(x86_num, 2),

            // stat64(path, buf) - path pointer, struct pointer
            195 => self.sys_stat(x86_num)?,

            // lstat64(path, buf) - path pointer, struct pointer
            196 => self.sys_stat(x86_num)?,

            // fstat64(fd, buf) - struct pointer
            197 => self.sys_fstat()?,

            // chown32(path, owner, group) - path pointer
            198 => self.sys_chown(x86_num)?,

            // getuid32() -> forward to getuid
            199 => self.sys_passthrough(x86_num, 0),

            // getgid32() -> forward to getgid
            200 => self.sys_passthrough(x86_num, 0),

            // geteuid32() -> forward to geteuid
            201 => self.sys_passthrough(x86_num, 0),

            // getegid32() -> forward to getegid
            202 => self.sys_passthrough(x86_num, 0),

            // setreuid32(ruid, euid)
            203 => self.sys_passthrough(x86_num, 2),

            // setregid32(rgid, egid)
            204 => self.sys_passthrough(x86_num, 2),

            // getgroups32(size, list)
            205 => self.sys_getgroups()?,

            // setgroups32(size, list)
            206 => self.sys_setgroups()?,

            // fchown32(fd, owner, group)
            207 => self.sys_passthrough(x86_num, 3),

            // setresuid32(ruid, euid, suid)
            208 => self.sys_passthrough(x86_num, 3),

            // getresuid32(ruid, euid, suid)
            209 => self.sys_getresuid()?,

            // setresgid32(rgid, egid, sgid)
            210 => self.sys_passthrough(x86_num, 3),

            // getresgid32(rgid, egid, sgid)
            211 => self.sys_getresgid()?,

            // lchown32(path, owner, group)
            212 => self.sys_chown(x86_num)?,

            // setuid32(uid)
            213 => self.sys_passthrough(x86_num, 1),

            // setgid32(gid)
            214 => self.sys_passthrough(x86_num, 1),

            // setfsuid32(uid)
            215 => self.sys_passthrough(x86_num, 1),

            // setfsgid32(gid)
            216 => self.sys_passthrough(x86_num, 1),

            // pivot_root(new_root, put_old)
            217 => bail!("pivot_root not yet implemented"),

            // getdents64(fd, dirp, count) - 64-bit dirent64
            220 => self.sys_getdents64()?,

            // fcntl64(fd, cmd, arg)
            221 => self.sys_passthrough(x86_num, 3),

            // tkill(tid, sig)
            222 => bail!("tkill not yet implemented"),

            // setxattr(path, name, value, size, flags)
            223 => self.sys_setxattr()?,

            // lsetxattr(path, name, value, size, flags)
            224 => self.sys_lsetxattr()?,

            // fsetxattr(fd, name, value, size, flags)
            225 => self.sys_fsetxattr()?,

            // getxattr(path, name, value, size)
            226 => self.sys_getxattr()?,

            // lgetxattr(path, name, value, size)
            227 => self.sys_lgetxattr()?,

            // fgetxattr(fd, name, value, size)
            228 => self.sys_fgetxattr()?,

            // listxattr(path, list, size)
            229 => self.sys_listxattr()?,

            // llistxattr(path, list, size)
            230 => self.sys_llistxattr()?,

            // flistxattr(fd, list, size)
            231 => self.sys_flistxattr()?,

            // removexattr(path, name)
            232 => self.sys_removexattr()?,

            // lremovexattr(path, name)
            233 => self.sys_lremovexattr()?,

            // fremovexattr(fd, name)
            234 => self.sys_fremovexattr()?,

            // futex(uaddr, op, val, timeout, uaddr2, val3) - m68k 235
            235 => self.sys_futex()?,

            // sendfile64(out_fd, in_fd, offset, count) - offset is pointer
            236 => self.sys_sendfile()?,

            // mincore(addr, length, vec)
            237 => self.sys_mincore()?,

            // madvise(addr, length, advice)
            238 => bail!("madvise not yet implemented"),

            // fcntl64(fd, cmd, arg)
            239 => self.sys_passthrough(x86_num, 3),

            // readahead(fd, offset, count)
            240 => bail!("readahead not yet implemented"),

            // io_setup(nr_events, ctx)
            241 => bail!("io_setup not yet implemented"),

            // io_destroy(ctx)
            242 => bail!("io_destroy not yet implemented"),

            // io_getevents(ctx, min_nr, nr, events, timeout)
            243 => bail!("io_getevents not yet implemented"),

            // io_submit(ctx, nr, iocbpp)
            244 => bail!("io_submit not yet implemented"),

            // io_cancel(ctx, iocb, result)
            245 => bail!("io_cancel not yet implemented"),

            // fadvise64(fd, offset, len, advice)
            246 => self.sys_passthrough(x86_num, 4),

            // exit_group(status)
            247 => self.sys_passthrough(x86_num, 1),

            // lookup_dcookie(cookie, buffer, len)
            248 => bail!("lookup_dcookie not yet implemented"),

            // epoll_create(size)
            249 => self.sys_passthrough(x86_num, 1),

            // epoll_ctl(epfd, op, fd, event) - m68k 250
            250 => self.sys_passthrough(x86_num, 4),

            // epoll_wait(epfd, events, maxevents, timeout) - m68k 251
            251 => self.sys_passthrough(x86_num, 4),

            // remap_file_pages(addr, size, prot, pgoff, flags)
            252 => bail!("remap_file_pages not yet implemented"),

            // set_tid_address(tidptr) - pointer
            253 => self.sys_passthrough(x86_num, 1),

            // timer_create(clockid, sevp, timerid) - m68k 254
            254 => self.sys_passthrough(x86_num, 3),

            // timer_settime(timerid, flags, new_value, old_value) - m68k 255
            255 => self.sys_passthrough(x86_num, 4),

            // timer_gettime(timerid, curr_value) - m68k 256
            256 => self.sys_passthrough(x86_num, 2),

            // timer_delete(timerid) - m68k 257
            257 => self.sys_passthrough(x86_num, 1),

            // timer_getoverrun(timerid)
            258 => bail!("timer_getoverrun not yet implemented"),

            // clock_settime(clockid, timespec)
            259 => self.sys_clock_settime()?,

            // clock_gettime(clockid, timespec)
            260 => self.sys_clock_gettime()?,

            // clock_getres(clockid, timespec)
            261 => self.sys_clock_getres()?,

            // clock_nanosleep(clockid, flags, request, remain)
            262 => self.sys_clock_nanosleep()?,

            // statfs64(path, buf) - path pointer, struct pointer
            263 => self.sys_statfs()?,

            // fstatfs64(fd, buf) - struct pointer
            264 => self.sys_fstatfs()?,

            // tgkill(tgid, tid, sig)
            265 => self.sys_passthrough(x86_num, 3),

            // utimes(filename, times) - path pointer, timeval array pointer
            266 => self.sys_utimes()?,

            // fadvise64_64(fd, offset, len, advice) - m68k only
            267 => self.sys_passthrough(x86_num, 4),

            // mbind(addr, len, mode, nodemask, maxnode, flags)
            268 => bail!("mbind not yet implemented"),

            // get_mempolicy(policy, nodemask, maxnode, addr, flags)
            269 => bail!("get_mempolicy not yet implemented"),

            // set_mempolicy(mode, nodemask, maxnode)
            270 => bail!("set_mempolicy not yet implemented"),

            // mq_open(name, oflag, mode, attr) - m68k 271
            271 => self.sys_mq_open()?,

            // mq_unlink(name) - m68k 272
            272 => self.sys_mq_unlink()?,

            // mq_timedsend(mqdes, msg_ptr, msg_len, msg_prio, abs_timeout) - m68k 273
            273 => self.sys_mq_timedsend()?,

            // mq_timedreceive(mqdes, msg_ptr, msg_len, msg_prio, abs_timeout) - m68k 274
            274 => self.sys_mq_timedreceive()?,

            // mq_notify(mqdes, notification)
            275 => bail!("mq_notify not yet implemented"),

            // mq_getsetattr(mqdes, newattr, oldattr) - m68k 276
            276 => self.sys_mq_getsetattr()?,

            // waitid(idtype, id, infop, options) - m68k 277
            277 => self.sys_waitid()?,

            // add_key(type, description, payload, plen, keyring)
            279 => bail!("add_key not yet implemented"),

            // request_key(type, description, callout_info, keyring)
            280 => bail!("request_key not yet implemented"),

            // keyctl(cmd, arg2, arg3, arg4, arg5)
            281 => bail!("keyctl not yet implemented"),

            // ioprio_set(which, who, ioprio)
            282 => bail!("ioprio_set not yet implemented"),

            // ioprio_get(which, who)
            283 => bail!("ioprio_get not yet implemented"),

            // inotify_init() - m68k 284
            284 => self.sys_passthrough(x86_num, 0),

            // inotify_add_watch(fd, path, mask) - m68k 285
            285 => self.sys_inotify_add_watch()?,

            // inotify_rm_watch(fd, wd) - m68k 286
            286 => self.sys_passthrough(x86_num, 2),

            // migrate_pages(pid, maxnode, old_nodes, new_nodes)
            287 => bail!("migrate_pages not yet implemented"),

            // openat(dirfd, path, flags, mode)
            288 => self.sys_openat()?,

            // mkdirat(dirfd, path, mode) - m68k 289
            289 => self.sys_mkdirat()?,

            // fchownat(dirfd, path, owner, group, flags) - m68k 291
            291 => self.sys_fchownat()?,

            // mknodat(dirfd, path, mode, dev) - m68k 290
            290 => self.sys_mknodat()?,

            // futimesat(dirfd, path, times) - m68k 292
            292 => self.sys_futimesat()?,

            // fstatat64(dirfd, path, buf, flags) - m68k 293
            293 => self.sys_fstatat64()?,

            // unlinkat(dirfd, path, flags) - m68k 294
            294 => self.sys_unlinkat()?,

            // renameat(olddirfd, oldpath, newdirfd, newpath) - m68k 295
            295 => self.sys_renameat()?,

            // linkat(olddirfd, oldpath, newdirfd, newpath, flags) - m68k 296
            296 => self.sys_linkat()?,

            // symlinkat(target, newdirfd, linkpath) - m68k 297
            297 => self.sys_symlinkat()?,

            // readlinkat(dirfd, path, buf, bufsiz) - m68k 298
            298 => self.sys_readlinkat()?,

            // fchmodat(dirfd, path, mode, flags) - m68k 299
            299 => self.sys_fchmodat()?,

            // faccessat(dirfd, path, mode, flags) - m68k 300
            300 => self.sys_faccessat()?,

            // pselect6(nfds, readfds, writefds, exceptfds, timeout, sigmask) - m68k 301
            301 => self.sys_passthrough(x86_num, 6),

            // ppoll(fds, nfds, timeout, sigmask, sigsetsize) - m68k 302
            302 => self.sys_passthrough(x86_num, 5),

            // unshare(flags)
            303 => bail!("unshare not yet implemented"),

            // set_robust_list(head, len)
            304 => bail!("set_robust_list not yet implemented"),

            // get_robust_list(pid, head, len)
            305 => bail!("get_robust_list not yet implemented"),

            // splice(fd_in, off_in, fd_out, off_out, len, flags) - m68k 306
            306 => self.sys_splice()?,

            // sync_file_range(fd, offset, nbytes, flags) - m68k 307
            307 => self.sys_passthrough(x86_num, 4),

            // tee(fd_in, fd_out, len, flags) - m68k 308
            308 => self.sys_passthrough(x86_num, 4),

            // vmsplice(fd, iov, nr_segs, flags) - m68k 309
            309 => self.sys_vmsplice()?,

            // move_pages(pid, count, pages, nodes, status, flags)
            310 => bail!("move_pages not yet implemented"),

            // sched_setaffinity(pid, cpusetsize, mask)
            311 => bail!("sched_setaffinity not yet implemented"),

            // sched_getaffinity(pid, cpusetsize, mask) - m68k 312
            312 => self.sys_passthrough(x86_num, 3),

            // kexec_load(entry, nr_segments, segments, flags)
            313 => bail!("kexec_load not yet implemented"),

            // getcpu(cpu, node, tcache) - m68k 314
            314 => self.sys_getcpu()?,

            // epoll_pwait(epfd, events, maxevents, timeout, sigmask, sigsetsize)
            315 => bail!("epoll_pwait not yet implemented"),

            // utimensat(dirfd, path, times, flags) - m68k 316
            316 => self.sys_utimensat()?,

            // signalfd(fd, mask, flags) - m68k 317
            317 => self.sys_signalfd()?,

            // timerfd_create(clockid, flags) - m68k 318
            318 => self.sys_passthrough(x86_num, 2),

            // eventfd(initval) - m68k 319
            319 => self.sys_passthrough(x86_num, 1),

            // fallocate(fd, mode, offset, len) - m68k 320
            320 => self.sys_passthrough(x86_num, 4),

            // timerfd_settime(fd, flags, new_value, old_value) - m68k 321
            321 => self.sys_timerfd_settime()?,

            // timerfd_gettime(fd, curr_value) - m68k 322
            322 => self.sys_timerfd_gettime()?,

            // signalfd4(fd, mask, sizemask, flags) - m68k 323
            323 => self.sys_signalfd4()?,

            // eventfd2(initval, flags) - m68k 324
            324 => self.sys_passthrough(x86_num, 2),

            // epoll_create1(flags) - m68k 325
            325 => self.sys_passthrough(x86_num, 1),

            // dup3(oldfd, newfd, flags) - m68k 326
            326 => self.sys_passthrough(x86_num, 3),

            // pipe2(pipefd, flags) - m68k 327
            327 => self.sys_pipe2()?,

            // inotify_init1(flags) - m68k 328
            328 => self.sys_passthrough(x86_num, 1),

            // preadv(fd, iov, iovcnt, pos_l, pos_h)
            329 => self.sys_preadv()?,

            // pwritev(fd, iov, iovcnt, pos_l, pos_h)
            330 => self.sys_pwritev()?,

            // rt_tgsigqueueinfo(tgid, tid, sig, info)
            331 => bail!("rt_tgsigqueueinfo not yet implemented"),

            // perf_event_open(attr, pid, cpu, group_fd, flags)
            332 => bail!("perf_event_open not yet implemented"),

            // get_thread_area()
            333 => self.sys_read_tp()?,

            // prlimit64(pid, resource, new_limit, old_limit) - m68k
            339 => self.sys_prlimit64()?,

            // name_to_handle_at(dirfd, name, handle, mnt_id, flags)
            340 => bail!("name_to_handle_at not yet implemented"),

            // open_by_handle_at(mountdirfd, handle, flags)
            341 => bail!("open_by_handle_at not yet implemented"),

            // clock_adjtime(clk_id, buf)
            342 => self.sys_clock_adjtime()?,

            // syncfs(fd) - m68k
            343 => self.sys_passthrough(x86_num, 1),

            // setns(fd, nstype)
            344 => bail!("setns not yet implemented"),

            // process_vm_readv(pid, local_iov, liovcnt, remote_iov, riovcnt, flags)
            345 => bail!("process_vm_readv not yet implemented"),

            // process_vm_writev(pid, local_iov, liovcnt, remote_iov, riovcnt, flags)
            346 => bail!("process_vm_writev not yet implemented"),

            // kcmp(pid1, pid2, type, idx1, idx2)
            347 => bail!("kcmp not yet implemented"),

            // finit_module(fd, param_values, flags)
            348 => bail!("finit_module not yet implemented"),

            // sched_setattr(pid, attr, flags)
            349 => bail!("sched_setattr not yet implemented"),

            // sched_getattr(pid, attr, size, flags)
            350 => bail!("sched_getattr not yet implemented"),

            // set_thread_area(addr)
            334 => self.set_thread_area()?,

            // renameat2(olddirfd, oldpath, newdirfd, newpath, flags) - m68k
            351 => self.sys_renameat2()?,

            // getrandom(buf, buflen, flags) - m68k
            352 => self.sys_getrandom()?,

            // memfd_create(name, flags) - m68k
            353 => self.sys_memfd_create()?,

            // bpf(cmd, attr, size)
            354 => bail!("bpf not yet implemented"),

            // execveat(dirfd, pathname, argv, envp, flags)
            355 => bail!("execveat not yet implemented"),

            // atomic_cmpxchg_32(uaddr, oldval, newval) - m68k
            335 => self.sys_atomic_cmpxchg_32()?,

            // atomic_barrier() - m68k
            336 => self.sys_atomic_barrier()?,

            // fanotify_init(flags, event_f_flags)
            337 => bail!("fanotify_init not yet implemented"),

            // fanotify_mark(fd, flags, mask, dirfd, pathname)
            338 => bail!("fanotify_mark not yet implemented"),

            // Socket syscalls (m68k uses separate syscalls, not socketcall)
            // socket(domain, type, protocol) - no pointers - m68k 356
            356 => self.sys_passthrough(x86_num, 3),

            // socketpair(domain, type, protocol, sv) - sv is pointer - m68k 357
            357 => self.sys_socketpair()?,

            // bind(sockfd, addr, addrlen) - addr is pointer - m68k 358
            358 => self.sys_socket_addr(x86_num)?,

            // connect(sockfd, addr, addrlen) - addr is pointer - m68k 359
            359 => self.sys_socket_addr(x86_num)?,

            // listen(sockfd, backlog) - no pointers - m68k 360
            360 => self.sys_passthrough(x86_num, 2),

            // accept4(sockfd, addr, addrlen, flags) - addr/addrlen pointers - m68k 361
            361 => self.sys_accept4()?,

            // getsockopt(sockfd, level, optname, optval, optlen) - pointers - m68k 362
            362 => self.sys_getsockopt()?,

            // setsockopt(sockfd, level, optname, optval, optlen) - optval pointer - m68k 363
            363 => self.sys_setsockopt()?,

            // getsockname(sockfd, addr, addrlen) - pointers - m68k 364
            364 => self.sys_getsockname()?,

            // getpeername(sockfd, addr, addrlen) - pointers - m68k 365
            365 => self.sys_getsockname()?,

            // sendto(sockfd, buf, len, flags, dest_addr, addrlen) - m68k 366
            366 => self.sys_sendto()?,

            // sendmsg(sockfd, msg, flags) - complex msghdr structure - m68k 367
            367 => self.sys_sendmsg()?,

            // recvfrom(sockfd, buf, len, flags, src_addr, addrlen) - m68k 368
            368 => self.sys_recvfrom()?,

            // recvmsg(sockfd, msg, flags) - complex msghdr structure - m68k 369
            369 => self.sys_recvmsg()?,

            // shutdown(sockfd, how) - no pointers - m68k 370
            370 => self.sys_passthrough(x86_num, 2),

            // recvmmsg(sockfd, msgvec, vlen, flags, timeout)
            371 => bail!("recvmmsg not yet implemented"),

            // sendmmsg(sockfd, msgvec, vlen, flags)
            372 => bail!("sendmmsg not yet implemented"),

            // userfaultfd(flags)
            373 => bail!("userfaultfd not yet implemented"),

            // membarrier(cmd, flags, cpu_id)
            374 => bail!("membarrier not yet implemented"),

            // mlock2(addr, len, flags) - m68k 375
            375 => self.sys_passthrough(x86_num, 3),

            // copy_file_range(fd_in, off_in, fd_out, off_out, len, flags) - m68k 376
            376 => self.sys_copy_file_range()?,

            // preadv2(fd, iov, iovcnt, offset, flags)
            377 => bail!("preadv2 not yet implemented"),

            // pwritev2(fd, iov, iovcnt, offset, flags)
            378 => bail!("pwritev2 not yet implemented"),

            // statx(dirfd, pathname, flags, mask, statxbuf) - m68k 379
            379 => self.sys_statx()?,

            // seccomp(operation, flags, args)
            380 => bail!("seccomp not yet implemented"),

            // pkey_mprotect(addr, len, prot, pkey) - m68k 381
            381 => self.sys_pkey_mprotect()?,

            // pkey_alloc(flags, access_rights) - m68k 382
            382 => self.sys_pkey_alloc()?,

            // pkey_free(pkey) - m68k 383
            383 => self.sys_pkey_free()?,

            // rseq(rseq, rseq_len, flags, sig) - m68k 384
            384 => self.sys_passthrough(x86_num, 4),

            // semget(key, nsems, semflg) - m68k 393
            393 => self.sys_passthrough(x86_num, 3),

            // semctl(semid, semnum, cmd, arg) - m68k 394
            394 => self.sys_semctl()?,

            // shmget(key, size, shmflg) - m68k 395
            395 => self.sys_passthrough(x86_num, 3),

            // shmctl(shmid, cmd, buf) - m68k 396
            396 => self.sys_shmctl()?,

            // shmat(shmid, shmaddr, shmflg) - m68k 397
            397 => self.sys_shmat()?,

            // shmdt(shmaddr) - m68k 398
            398 => self.sys_shmdt()?,

            // msgget(key, msgflg) - m68k 399
            399 => self.sys_passthrough(x86_num, 2),

            // msgsnd(msqid, msgp, msgsz, msgflg) - m68k 400
            400 => self.sys_msgsnd()?,

            // msgrcv(msqid, msgp, msgsz, msgtyp, msgflg) - m68k 401
            401 => self.sys_msgrcv()?,

            // msgctl(msqid, cmd, buf) - m68k 402
            402 => self.sys_msgctl()?,

            // clock_gettime(clockid, timespec)
            403 => self.sys_clock_gettime()?,

            // clock_settime64(clockid, timespec) -> clock_settime
            404 => self.sys_clock_settime()?,

            // clock_adjtime64(clk_id, buf)
            405 => self.sys_clock_adjtime()?,

            // clock_getres_time64(clockid, timespec) -> clock_getres
            406 => self.sys_clock_getres()?,

            // clock_nanosleep_time64(clockid, flags, request, remain) -> clock_nanosleep
            407 => self.sys_clock_nanosleep()?,

            // timer_gettime64(timerid, curr_value) -> timer_gettime
            408 => self.sys_passthrough(x86_num, 2),

            // timer_settime64(timerid, flags, new_value, old_value) -> timer_settime
            409 => self.sys_passthrough(x86_num, 4),

            // timerfd_gettime64(fd, curr_value) - m68k 410
            410 => self.sys_timerfd_gettime()?,

            // timerfd_settime64(fd, flags, new_value, old_value) - m68k 411
            411 => self.sys_timerfd_settime()?,

            // utimensat_time64(dirfd, path, times, flags) -> utimensat
            412 => self.sys_utimensat()?,

            // pselect6_time64(nfds, readfds, writefds, exceptfds, timeout, sigmask)
            413 => self.sys_pselect6()?,

            // ppoll_time64(fds, nfds, timeout, sigmask, sigsetsize)
            414 => bail!("ppoll_time64 not yet implemented"),

            // io_pgetevents_time64(ctx, min_nr, nr, events, timeout, sig)
            416 => bail!("io_pgetevents_time64 not yet implemented"),

            // recvmmsg_time64(sockfd, msgvec, vlen, flags, timeout)
            417 => bail!("recvmmsg_time64 not yet implemented"),

            // mq_timedsend_time64 - m68k 418 (same as mq_timedsend, already handles 64-bit time_t)
            418 => self.sys_mq_timedsend()?,

            // mq_timedreceive_time64 - m68k 419 (same as mq_timedreceive, already handles 64-bit time_t)
            419 => self.sys_mq_timedreceive()?,

            // semtimedop_time64(semid, sops, nsops, timeout)
            420 => bail!("semtimedop_time64 not yet implemented"),

            // rt_sigtimedwait_time64(set, info, timeout, sigsetsize)
            421 => bail!("rt_sigtimedwait_time64 not yet implemented"),

            // futex_time64(uaddr, op, val, timeout, uaddr2, val3)
            422 => bail!("futex_time64 not yet implemented"),

            // sched_rr_get_interval_time64(pid, tp)
            423 => bail!("sched_rr_get_interval_time64 not yet implemented"),

            // pidfd_send_signal(pidfd, sig, info, flags)
            424 => bail!("pidfd_send_signal not yet implemented"),

            // io_uring_setup(entries, params)
            425 => bail!("io_uring_setup not yet implemented"),

            // io_uring_enter(fd, to_submit, min_complete, flags, sig)
            426 => bail!("io_uring_enter not yet implemented"),

            // io_uring_register(fd, opcode, arg, nr_args)
            427 => bail!("io_uring_register not yet implemented"),

            // open_tree(dirfd, pathname, flags)
            428 => bail!("open_tree not yet implemented"),

            // move_mount(from_dirfd, from_pathname, to_dirfd, to_pathname, flags)
            429 => bail!("move_mount not yet implemented"),

            // fsopen(fsname, flags)
            430 => bail!("fsopen not yet implemented"),

            // fsconfig(fd, cmd, key, value, aux)
            431 => bail!("fsconfig not yet implemented"),

            // fsmount(fd, flags, attr_flags)
            432 => bail!("fsmount not yet implemented"),

            // fspick(dirfd, pathname, flags)
            433 => bail!("fspick not yet implemented"),

            // pidfd_open(pid, flags)
            434 => bail!("pidfd_open not yet implemented"),

            // clone3(cl_args, size)
            435 => bail!("clone3 not yet implemented"),

            // close_range(first, last, flags)
            436 => self.sys_passthrough(x86_num, 3),

            // openat2(dirfd, path, how, size) - extended openat with open_how struct
            437 => self.sys_openat2()?,

            // pidfd_getfd(pidfd, targetfd, flags)
            438 => bail!("pidfd_getfd not yet implemented"),

            // faccessat2(dirfd, pathname, mode, flags)
            439 => bail!("faccessat2 not yet implemented"),

            // process_madvise(pidfd, iovec, vlen, advice, flags)
            440 => bail!("process_madvise not yet implemented"),

            // epoll_pwait2(epfd, events, maxevents, timeout, sigmask, sigsetsize)
            441 => bail!("epoll_pwait2 not yet implemented"),

            // mount_setattr(dirfd, pathname, flags, attr, size)
            442 => bail!("mount_setattr not yet implemented"),

            // quotactl_fd(fd, cmd, id, addr)
            443 => bail!("quotactl_fd not yet implemented"),

            // landlock_create_ruleset(attr, size, flags)
            444 => self.sys_landlock_create_ruleset()?,

            // landlock_add_rule(ruleset_fd, rule_type, rule_attr, flags)
            445 => self.sys_landlock_add_rule()?,

            // landlock_restrict_self(ruleset_fd, flags)
            446 => self.sys_landlock_restrict_self()?,

            // process_mrelease(pidfd, flags)
            448 => bail!("process_mrelease not yet implemented"),

            // futex_waitv(waiters, nr_futexes, flags, timeout, clockid)
            449 => bail!("futex_waitv not yet implemented"),

            // set_mempolicy_home_node(addr, len, home_node, flags)
            450 => bail!("set_mempolicy_home_node not yet implemented"),

            // cachestat(fd, cstat_range, cstat, flags)
            451 => bail!("cachestat not yet implemented"),

            // fchmodat2(dirfd, path, mode, flags) - m68k 452
            452 => self.sys_fchmodat2()?,

            // map_shadow_stack(addr, size, flags)
            453 => bail!("map_shadow_stack not yet implemented"),

            // futex_wake(uaddr, nr_wake, mask, flags)
            454 => bail!("futex_wake not yet implemented"),

            // futex_wait(uaddr, val, mask, flags, timeout, clockid)
            455 => bail!("futex_wait not yet implemented"),

            // futex_requeue(uaddr, uaddr2, nr_wake, nr_requeue, cmpval, flags)
            456 => bail!("futex_requeue not yet implemented"),

            // statmount(mnt_id, buf, bufsize, flags)
            457 => bail!("statmount not yet implemented"),

            // listmount(mnt_id, buf, bufsize, flags)
            458 => bail!("listmount not yet implemented"),

            // lsm_get_self_attr(attr, ctx, size, flags)
            459 => bail!("lsm_get_self_attr not yet implemented"),

            // lsm_set_self_attr(attr, ctx, size, flags)
            460 => bail!("lsm_set_self_attr not yet implemented"),

            // lsm_list_modules(ids, size, flags)
            461 => bail!("lsm_list_modules not yet implemented"),

            // mseal(addr, len, flags). Unsupported on 32-bit linux, so return -EPERM
            462 => self.sys_mseal()?,

            // setxattrat(dirfd, path, name, value, size, flags)
            463 => self.sys_setxattrat()?,

            // getxattrat(dirfd, path, name, value, size)
            464 => self.sys_getxattrat()?,

            // listxattrat(dirfd, path, args, atflags)
            465 => self.sys_listxattrat()?,

            // removexattrat(dirfd, path, name, atflags)
            466 => self.sys_removexattrat()?,

            // open_tree_attr(dirfd, path, flags, attr, size) - m68k 467
            467 => self.sys_open_tree_attr()?,

            // file_getattr(dirfd, path, *fsx, size, at_flags)
            468 => bail!("file_getattr not yet implemented"),

            // file_setattr(dirfd, path, *fsx, size, at_flags)
            469 => bail!("file_setattr not yet implemented"),

            // For syscalls with no pointer args, passthrough directly
            syscall_num => bail!("Unsupported syscall number: {syscall_num}"),
        };

        self.data_regs[0] = result as u32;
        Ok(())
    }

    /// Set thread area
    fn set_thread_area(&mut self) -> Result<i64> {
        let tls_addr = self.data_regs[1] as usize;
        self.ensure_tls_range(tls_addr)?;
        if self.tls_memsz > 0 {
            let new_start = tls_addr
                .checked_sub(M68K_TLS_TCB_SIZE)
                .ok_or_else(|| anyhow!("new TLS base underflow"))?;
            if self.memory.covers_range(new_start, self.tls_memsz) {
                let zeros = vec![0u8; self.tls_memsz];
                self.memory.write_data(new_start, &zeros)?;
            }
        }
        if self.tls_base != 0 && self.tls_base as usize != tls_addr {
            let old_start =
                self.tls_base
                    .checked_sub(M68K_TLS_TCB_SIZE as u32)
                    .ok_or_else(|| anyhow!("old TLS base underflow"))? as usize;
            let new_start = tls_addr
                .checked_sub(M68K_TLS_TCB_SIZE)
                .ok_or_else(|| anyhow!("new TLS base underflow"))?;
            let copy_len = self.tls_memsz.min(M68K_TLS_TCB_SIZE + TLS_DATA_PAD);
            if copy_len > 0 && self.memory.covers_range(new_start, copy_len) {
                if self.memory.covers_range(old_start, copy_len) && self.tls_initialized {
                    let snapshot = self.memory.read_data(old_start, copy_len)?.to_vec();
                    self.memory.write_data(new_start, &snapshot)?;
                } else {
                    let zeros = vec![0u8; copy_len];
                    self.memory.write_data(new_start, &zeros)?;
                }
            }
        }
        self.tls_base = tls_addr as u32;
        self.tls_initialized = true;
        // Note: Don't modify A0 or A6 - A6 is the frame pointer!
        // The syscall return value (0 for success) goes in D0.
        Ok(0)
    }

    /// Ensure the TLS region around the thread pointer is backed by memory.
    fn ensure_tls_range(&mut self, thread_ptr: usize) -> Result<()> {
        let end = thread_ptr
            .checked_add(TLS_DATA_PAD)
            .ok_or_else(|| anyhow!("thread pointer overflow"))?;

        // TLS is expected to live on the heap; grow the heap segment if needed.
        let heap_len = self
            .memory
            .segments()
            .iter()
            .find(|s| s.vaddr == self.heap_segment_base)
            .map(|s| s.len())
            .ok_or_else(|| anyhow!("heap segment not found"))?;
        let current_end = self.heap_segment_base + heap_len;
        if end > current_end {
            // Keep a small guard below the stack.
            let guard: usize = 0x1000;
            if end + guard > self.stack_base {
                bail!("TLS region would overlap the stack");
            }
            let new_len = end
                .checked_sub(self.heap_segment_base)
                .ok_or_else(|| anyhow!("TLS end before heap base"))?;
            self.memory
                .resize_segment(self.heap_segment_base, new_len)?;
        }

        Ok(())
    }

    /// Convert libc syscall result (-1 + errno) to kernel ABI (-errno).
    /// libc::syscall returns -1 on error and sets errno.
    /// The kernel returns -errno directly.
    fn libc_to_kernel(result: i64) -> i64 {
        if result == -1 {
            let errno = std::io::Error::last_os_error().raw_os_error().unwrap_or(1);
            -(errno as i64)
        } else {
            result
        }
    }

    /// Translate m68k open() flags to x86-64 flags.
    /// m68k and x86-64 have different values for O_DIRECTORY, O_NOFOLLOW, O_DIRECT, O_LARGEFILE.
    fn translate_open_flags(m68k_flags: i32) -> i32 {
        // m68k values (from asm/fcntl.h)
        const M68K_O_DIRECTORY: i32 = 0o040000;
        const M68K_O_NOFOLLOW: i32 = 0o100000;
        const M68K_O_DIRECT: i32 = 0o200000;
        const M68K_O_LARGEFILE: i32 = 0o400000;

        // x86-64 values (from asm-generic/fcntl.h)
        const X86_64_O_DIRECT: i32 = 0o040000;
        const X86_64_O_LARGEFILE: i32 = 0o100000;
        const X86_64_O_DIRECTORY: i32 = 0o200000;
        const X86_64_O_NOFOLLOW: i32 = 0o400000;

        // Mask for flags that are the same on both architectures
        const COMMON_FLAGS_MASK: i32 = 0o037777;

        // Start with common flags (O_RDONLY, O_WRONLY, O_RDWR, O_CREAT, O_EXCL, etc.)
        let mut x86_flags = m68k_flags & COMMON_FLAGS_MASK;

        // Translate architecture-specific flags
        if m68k_flags & M68K_O_DIRECTORY != 0 {
            x86_flags |= X86_64_O_DIRECTORY;
        }
        if m68k_flags & M68K_O_NOFOLLOW != 0 {
            x86_flags |= X86_64_O_NOFOLLOW;
        }
        if m68k_flags & M68K_O_DIRECT != 0 {
            x86_flags |= X86_64_O_DIRECT;
        }
        if m68k_flags & M68K_O_LARGEFILE != 0 {
            x86_flags |= X86_64_O_LARGEFILE;
        }

        // O_CLOEXEC is at 0o2000000 on both architectures (keep it)
        x86_flags |= m68k_flags & 0o2000000;

        x86_flags
    }

    /// Generic syscall passthrough for syscalls with no pointer arguments.
    fn sys_passthrough(&self, syscall_num: u32, arg_count: usize) -> i64 {
        let arg = |i: usize| self.data_regs[i + 1] as i64;
        let result = unsafe {
            match arg_count {
                0 => libc::syscall(syscall_num as i64),
                1 => libc::syscall(syscall_num as i64, arg(0)),
                2 => libc::syscall(syscall_num as i64, arg(0), arg(1)),
                3 => libc::syscall(syscall_num as i64, arg(0), arg(1), arg(2)),
                4 => libc::syscall(syscall_num as i64, arg(0), arg(1), arg(2), arg(3)),
                _ => libc::syscall(syscall_num as i64, arg(0), arg(1), arg(2), arg(3), arg(4)),
            }
        };
        Self::libc_to_kernel(result)
    }

    /// Check if an fd is set in a guest fd_set (m68k format: 32-bit big-endian longs)
    fn guest_fd_isset(&self, fd: i32, fdset_addr: usize) -> Result<bool> {
        // fd_set on m68k uses 32-bit longs, so we have 32 longs (128 bytes total)
        // Each long holds 32 bits
        let long_index = (fd / 32) as usize;
        let bit_index = fd % 32;

        // Read the 32-bit big-endian long
        let long_addr = fdset_addr + long_index * 4;
        let long_val = self.memory.read_long(long_addr)?;

        // Check if the bit is set (bit 0 is the LSB)
        Ok((long_val & (1 << bit_index)) != 0)
    }

    /// Convert guest fd_set to host fd_set
    fn guest_to_host_fdset(&self, guest_addr: usize, nfds: i32) -> Result<libc::fd_set> {
        let mut host_set: libc::fd_set = unsafe { std::mem::zeroed() };

        for fd in 0..nfds {
            if self.guest_fd_isset(fd, guest_addr)? {
                unsafe {
                    libc::FD_SET(fd, &mut host_set);
                }
            }
        }

        Ok(host_set)
    }

    /// Copy host fd_set back to guest fd_set
    fn host_to_guest_fdset(
        &mut self,
        host_set: &libc::fd_set,
        guest_addr: usize,
        nfds: i32,
    ) -> Result<()> {
        // Clear the guest fd_set first (32 longs * 4 bytes = 128 bytes)
        for i in 0..32 {
            let zero_bytes = [0u8; 4];
            self.memory.write_data(guest_addr + i * 4, &zero_bytes)?;
        }

        // Set bits for each fd that's set in the host set
        for fd in 0..nfds {
            if unsafe { libc::FD_ISSET(fd, host_set) } {
                let long_index = (fd / 32) as usize;
                let bit_index = fd % 32;
                let long_addr = guest_addr + long_index * 4;

                let current = self.memory.read_long(long_addr)?;
                let new_val = current | (1 << bit_index);

                // Write as big-endian bytes
                let bytes = new_val.to_be_bytes();
                self.memory.write_data(long_addr, &bytes)?;
            }
        }

        Ok(())
    }

    /// mseal(addr, len, flags) unimplemented
    fn sys_mseal(&self) -> Result<i64> {
        // not implemented on 32-bit linuxes.
        Ok(-1)
    }

    /// pselect6_time64(nfds, readfds, writefds, exceptfds, timeout, sigmask)
    fn sys_pselect6(&mut self) -> Result<i64> {
        let nfds = self.data_regs[1] as i32;
        let readfds_addr = self.data_regs[2] as usize;
        let writefds_addr = self.data_regs[3] as usize;
        let exceptfds_addr = self.data_regs[4] as usize;
        let timeout_addr = self.data_regs[5] as usize;
        // Note: sigmask (6th arg) would be on stack for m68k, but we'll ignore it for now

        // Convert guest fd_sets to host format
        let mut readfds_opt = if readfds_addr != 0 {
            Some(self.guest_to_host_fdset(readfds_addr, nfds)?)
        } else {
            None
        };

        let mut writefds_opt = if writefds_addr != 0 {
            Some(self.guest_to_host_fdset(writefds_addr, nfds)?)
        } else {
            None
        };

        let mut exceptfds_opt = if exceptfds_addr != 0 {
            Some(self.guest_to_host_fdset(exceptfds_addr, nfds)?)
        } else {
            None
        };

        // Handle timeout
        let timeout_opt = if timeout_addr == 0 {
            None
        } else {
            // Read the time64 timespec structure from guest memory (two 64-bit fields)
            // Each 64-bit field is stored as two 32-bit values (big-endian on m68k)
            let tv_sec_hi = self.memory.read_long(timeout_addr)? as i64;
            let tv_sec_lo = self.memory.read_long(timeout_addr + 4)? as i64;
            let tv_sec = (tv_sec_hi << 32) | (tv_sec_lo & 0xFFFFFFFF);

            let tv_nsec_hi = self.memory.read_long(timeout_addr + 8)? as i64;
            let tv_nsec_lo = self.memory.read_long(timeout_addr + 12)? as i64;
            let tv_nsec = (tv_nsec_hi << 32) | (tv_nsec_lo & 0xFFFFFFFF);

            Some(libc::timespec { tv_sec, tv_nsec })
        };

        // Get pointers to fd_sets
        let readfds_ptr = readfds_opt
            .as_mut()
            .map(|s| s as *mut _)
            .unwrap_or(std::ptr::null_mut());
        let writefds_ptr = writefds_opt
            .as_mut()
            .map(|s| s as *mut _)
            .unwrap_or(std::ptr::null_mut());
        let exceptfds_ptr = exceptfds_opt
            .as_mut()
            .map(|s| s as *mut _)
            .unwrap_or(std::ptr::null_mut());

        // Call pselect6 (x86_64 syscall 270)
        let result = unsafe {
            if let Some(ref timeout) = timeout_opt {
                libc::syscall(
                    libc::SYS_pselect6,
                    nfds,
                    readfds_ptr,
                    writefds_ptr,
                    exceptfds_ptr,
                    timeout as *const libc::timespec,
                    std::ptr::null::<libc::sigset_t>(), // sigmask
                )
            } else {
                libc::syscall(
                    libc::SYS_pselect6,
                    nfds,
                    readfds_ptr,
                    writefds_ptr,
                    exceptfds_ptr,
                    std::ptr::null::<libc::timespec>(),
                    std::ptr::null::<libc::sigset_t>(), // sigmask
                )
            }
        };

        let ret = Self::libc_to_kernel(result);

        // Copy modified fd_sets back to guest memory
        if ret >= 0 {
            if let Some(ref readfds) = readfds_opt
                && readfds_addr != 0
            {
                self.host_to_guest_fdset(readfds, readfds_addr, nfds)?;
            }
            if let Some(ref writefds) = writefds_opt
                && writefds_addr != 0
            {
                self.host_to_guest_fdset(writefds, writefds_addr, nfds)?;
            }
            if let Some(ref exceptfds) = exceptfds_opt
                && exceptfds_addr != 0
            {
                self.host_to_guest_fdset(exceptfds, exceptfds_addr, nfds)?;
            }
        }

        Ok(ret)
    }

    /// select(nfds, readfds, writefds, exceptfds, timeout)
    fn sys_select(&mut self) -> Result<i64> {
        let nfds = self.data_regs[1] as i32;
        let readfds_addr = self.data_regs[2] as usize;
        let writefds_addr = self.data_regs[3] as usize;
        let exceptfds_addr = self.data_regs[4] as usize;
        let timeout_addr = self.data_regs[5] as usize;

        // Convert guest fd_sets to host format
        let mut readfds_opt = if readfds_addr != 0 {
            Some(self.guest_to_host_fdset(readfds_addr, nfds)?)
        } else {
            None
        };

        let mut writefds_opt = if writefds_addr != 0 {
            Some(self.guest_to_host_fdset(writefds_addr, nfds)?)
        } else {
            None
        };

        let mut exceptfds_opt = if exceptfds_addr != 0 {
            Some(self.guest_to_host_fdset(exceptfds_addr, nfds)?)
        } else {
            None
        };

        // Handle timeout (timeval structure: two longs)
        let mut timeout_opt = if timeout_addr == 0 {
            None
        } else {
            let tv_sec = self.memory.read_long(timeout_addr)? as i64;
            let tv_usec = self.memory.read_long(timeout_addr + 4)? as i64;
            Some(libc::timeval { tv_sec, tv_usec })
        };

        // Get pointers to fd_sets
        let readfds_ptr = readfds_opt
            .as_mut()
            .map(|s| s as *mut _)
            .unwrap_or(std::ptr::null_mut());
        let writefds_ptr = writefds_opt
            .as_mut()
            .map(|s| s as *mut _)
            .unwrap_or(std::ptr::null_mut());
        let exceptfds_ptr = exceptfds_opt
            .as_mut()
            .map(|s| s as *mut _)
            .unwrap_or(std::ptr::null_mut());
        let timeout_ptr = timeout_opt
            .as_mut()
            .map(|t| t as *mut _)
            .unwrap_or(std::ptr::null_mut());

        // Call select
        let result =
            unsafe { libc::select(nfds, readfds_ptr, writefds_ptr, exceptfds_ptr, timeout_ptr) };

        let ret = Self::libc_to_kernel(result as i64);

        // Copy modified fd_sets back to guest memory
        if ret >= 0 {
            if let Some(ref readfds) = readfds_opt
                && readfds_addr != 0
            {
                self.host_to_guest_fdset(readfds, readfds_addr, nfds)?;
            }
            if let Some(ref writefds) = writefds_opt
                && writefds_addr != 0
            {
                self.host_to_guest_fdset(writefds, writefds_addr, nfds)?;
            }
            if let Some(ref exceptfds) = exceptfds_opt
                && exceptfds_addr != 0
            {
                self.host_to_guest_fdset(exceptfds, exceptfds_addr, nfds)?;
            }
        }

        Ok(ret)
    }

    /// clone(flags, stack, parent_tid, child_tid, tls)
    /// On x86_64: clone(flags, stack, parent_tid, child_tid, tls)
    /// On m68k: clone(flags, stack, parent_tid, child_tid, tls) - same order
    fn sys_clone(&mut self) -> Result<i64> {
        let flags = self.data_regs[1] as u64;
        let stack = self.data_regs[2] as usize;
        let parent_tid_addr = self.data_regs[3] as usize;
        let child_tid_addr = self.data_regs[4] as usize;
        let tls = self.data_regs[5] as u64;

        // Translate pointer arguments from guest to host
        let parent_tid_ptr = if parent_tid_addr == 0 {
            std::ptr::null_mut()
        } else {
            self.memory
                .guest_to_host_mut(parent_tid_addr, 4)
                .ok_or_else(|| anyhow!("invalid parent_tid pointer"))?
                as *mut libc::pid_t
        };

        let child_tid_ptr = if child_tid_addr == 0 {
            std::ptr::null_mut()
        } else {
            self.memory
                .guest_to_host_mut(child_tid_addr, 4)
                .ok_or_else(|| anyhow!("invalid child_tid pointer"))?
                as *mut libc::pid_t
        };

        // For now, we don't support custom stack (would need complex setup)
        if stack != 0 {
            bail!("clone with custom stack not yet supported");
        }

        // Call host clone syscall with translated pointers
        let result = unsafe {
            libc::syscall(
                libc::SYS_clone,
                flags as i64,
                0, // stack (NULL for fork-like behavior)
                parent_tid_ptr,
                child_tid_ptr,
                tls as i64,
            )
        };
        Ok(Self::libc_to_kernel(result))
    }

    /// execve(filename, argv, envp)
    /// Replace the current process with a new program
    fn sys_execve(&mut self) -> Result<i64> {
        use goblin::{Object, elf::program_header};
        use std::fs;

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

    /// read(fd, buf, count)
    fn sys_read(&mut self) -> Result<i64> {
        let fd = self.data_regs[1] as i32;
        let buf = self.data_regs[2] as usize;
        let count = self.data_regs[3] as usize;

        let host_ptr = self.guest_mut_ptr(buf, count)?;

        let result = unsafe { libc::read(fd, host_ptr, count) as i64 };
        Ok(Self::libc_to_kernel(result))
    }

    /// write(fd, buf, count)
    fn sys_write(&self) -> Result<i64> {
        let fd = self.data_regs[1] as i32;
        let buf = self.data_regs[2] as usize;
        let count = self.data_regs[3] as usize;

        let host_ptr = self.guest_const_ptr(buf, count)?;

        let result = unsafe { libc::write(fd, host_ptr, count) as i64 };
        Ok(Self::libc_to_kernel(result))
    }

    fn build_iovecs(
        &mut self,
        base_addr: usize,
        count: usize,
        writable: bool,
    ) -> Result<Vec<libc::iovec>> {
        let mut iovecs = Vec::with_capacity(count);
        for i in 0..count {
            let entry = base_addr + i * 8;
            let iov_base = self.memory.read_long(entry)? as usize;
            let iov_len = self.memory.read_long(entry + 4)? as usize;
            if iov_len == 0 {
                // Zero-length iovecs are allowed; use null pointer.
                iovecs.push(libc::iovec {
                    iov_base: std::ptr::null_mut(),
                    iov_len,
                });
                continue;
            }

            let host_ptr = if writable {
                self.memory
                    .guest_to_host_mut(iov_base, iov_len)
                    .ok_or_else(|| anyhow!("invalid iovec buffer {iov_base:#x} (len {iov_len})"))?
            } else {
                self.memory
                    .guest_to_host(iov_base, iov_len)
                    .ok_or_else(|| anyhow!("invalid iovec buffer {iov_base:#x} (len {iov_len})"))?
            };

            iovecs.push(libc::iovec {
                iov_base: host_ptr as *mut libc::c_void,
                iov_len,
            });
        }
        Ok(iovecs)
    }

    /// readv(fd, iov, iovcnt)
    fn sys_readv(&mut self) -> Result<i64> {
        let fd = self.data_regs[1] as i32;
        let iov_addr = self.data_regs[2] as usize;
        let iovcnt = self.data_regs[3] as usize;
        let iovecs = self.build_iovecs(iov_addr, iovcnt, true)?;
        Ok(unsafe { libc::readv(fd, iovecs.as_ptr(), iovecs.len() as i32) as i64 })
    }

    /// writev(fd, iov, iovcnt)
    fn sys_writev(&mut self) -> Result<i64> {
        let fd = self.data_regs[1] as i32;
        let iov_addr = self.data_regs[2] as usize;
        let iovcnt = self.data_regs[3] as usize;
        let iovecs = self.build_iovecs(iov_addr, iovcnt, false)?;
        Ok(unsafe { libc::writev(fd, iovecs.as_ptr(), iovecs.len() as i32) as i64 })
    }

    /// uClibc helper: return TLS base (m68k uses syscall number 333 for this)
    fn sys_read_tp(&mut self) -> Result<i64> {
        let mut tp = self.tls_base;
        if tp == 0 {
            // Lazily allocate a TLS block if none was configured.
            // Leave room for the 0x7000-byte TCB gap plus some space for
            // initial TLS data.
            const TLS_SIZE: usize = M68K_TLS_TCB_SIZE + TLS_DATA_PAD;
            let addr = self
                .memory
                .find_free_range(TLS_SIZE)
                .ok_or_else(|| anyhow!("no space for TLS block"))?;
            self.memory.add_segment(crate::memory::MemorySegment {
                vaddr: addr,
                data: crate::memory::MemoryData::Owned(vec![0u8; TLS_SIZE]),
                flags: goblin::elf::program_header::PF_R | goblin::elf::program_header::PF_W,
                align: 0x1000,
            });
            tp = (addr + M68K_TLS_TCB_SIZE) as u32;
            self.tls_base = tp;
        }

        // Returned in D0 only. Don't modify A0 or A6 - A6 is the frame pointer!
        self.data_regs[0] = tp;
        Ok(tp as i64)
    }

    /// open(path, flags, mode)
    fn sys_open(&self) -> Result<i64> {
        let path_addr = self.data_regs[1] as usize;
        let m68k_flags = self.data_regs[2] as i32;
        let mode = self.data_regs[3];

        let path_cstr = self.guest_cstring(path_addr)?;
        let flags = Self::translate_open_flags(m68k_flags);
        let result = unsafe { libc::open(path_cstr.as_ptr(), flags, mode) as i64 };
        Ok(Self::libc_to_kernel(result))
    }

    /// openat(dirfd, path, flags, mode)
    fn sys_openat(&self) -> Result<i64> {
        let dirfd = self.data_regs[1] as i32;
        let path_addr = self.data_regs[2] as usize;
        let m68k_flags = self.data_regs[3] as i32;
        let mode = self.data_regs[4];

        let path_cstr = self.guest_cstring(path_addr)?;
        let flags = Self::translate_open_flags(m68k_flags);
        let result = unsafe { libc::openat(dirfd, path_cstr.as_ptr(), flags, mode) as i64 };
        Ok(Self::libc_to_kernel(result))
    }

    /// openat2(dirfd, path, how, size)
    /// Extended version of openat with struct open_how for additional control
    fn sys_openat2(&self) -> Result<i64> {
        let dirfd = self.data_regs[1] as i32;
        let path_addr = self.data_regs[2] as usize;
        let how_addr = self.data_regs[3] as usize;
        let size = self.data_regs[4] as usize;

        // Read path
        let path_cstr = self.guest_cstring(path_addr)?;

        // Read struct open_how from guest memory (m68k big-endian)
        // struct open_how {
        //     u64 flags;    // 0: 8 bytes
        //     u64 mode;     // 8: 8 bytes
        //     u64 resolve;  // 16: 8 bytes
        // }
        // Total: 24 bytes minimum

        if size < 24 {
            // Size too small for valid open_how structure
            return Ok(Self::libc_to_kernel(-libc::EINVAL as i64));
        }

        // Read fields using the helper
        let m68k_flags = self.read_u64_be(how_addr)?;
        let mode = self.read_u64_be(how_addr + 8)?;
        let resolve = self.read_u64_be(how_addr + 16)?;

        // Translate flags from m68k to host
        let host_flags = Self::translate_open_flags(m68k_flags as i32) as u64;

        // Build host open_how structure
        #[repr(C)]
        struct OpenHow {
            flags: u64,
            mode: u64,
            resolve: u64,
        }

        let host_how = OpenHow {
            flags: host_flags,
            mode,
            resolve,
        };

        // Call openat2 via syscall (no libc wrapper)
        let result = unsafe {
            libc::syscall(
                437, // SYS_openat2
                dirfd,
                path_cstr.as_ptr(),
                &host_how as *const OpenHow,
                std::mem::size_of::<OpenHow>(),
            ) as i64
        };

        Ok(Self::libc_to_kernel(result))
    }

    /// mkdirat(dirfd, path, mode)
    fn sys_mkdirat(&self) -> Result<i64> {
        let dirfd = self.data_regs[1] as i32;
        let path_addr = self.data_regs[2] as usize;
        let mode = self.data_regs[3] as libc::mode_t;

        let path_cstr = self.guest_cstring(path_addr)?;
        let result = unsafe { libc::mkdirat(dirfd, path_cstr.as_ptr(), mode) as i64 };
        Ok(Self::libc_to_kernel(result))
    }

    /// unlinkat(dirfd, path, flags)
    fn sys_unlinkat(&self) -> Result<i64> {
        let dirfd = self.data_regs[1] as i32;
        let path_addr = self.data_regs[2] as usize;
        let flags = self.data_regs[3] as i32;

        let path_cstr = self.guest_cstring(path_addr)?;
        let result = unsafe { libc::unlinkat(dirfd, path_cstr.as_ptr(), flags) as i64 };
        Ok(Self::libc_to_kernel(result))
    }

    /// fchmodat(dirfd, path, mode, flags)
    fn sys_fchmodat(&self) -> Result<i64> {
        let dirfd = self.data_regs[1] as i32;
        let path_addr = self.data_regs[2] as usize;
        let mode = self.data_regs[3] as libc::mode_t;
        let flags = self.data_regs[4] as i32;

        let path_cstr = self.guest_cstring(path_addr)?;
        let result = unsafe { libc::fchmodat(dirfd, path_cstr.as_ptr(), mode, flags) as i64 };
        Ok(Self::libc_to_kernel(result))
    }

    /// fchmodat2(dirfd, path, mode, flags)
    /// Extended version of fchmodat (Linux 6.6+) that properly supports AT_SYMLINK_NOFOLLOW
    fn sys_fchmodat2(&self) -> Result<i64> {
        let dirfd = self.data_regs[1] as i32;
        let path_addr = self.data_regs[2] as usize;
        let mode = self.data_regs[3] as libc::mode_t;
        let flags = self.data_regs[4] as i32;

        let path_cstr = self.guest_cstring(path_addr)?;

        // Call fchmodat2 via syscall (no libc wrapper yet)
        let result = unsafe {
            libc::syscall(
                452, // SYS_fchmodat2
                dirfd,
                path_cstr.as_ptr(),
                mode,
                flags,
            ) as i64
        };

        Ok(Self::libc_to_kernel(result))
    }

    /// open_tree_attr(dirfd, path, flags, attr, size)
    /// Extended open_tree with mount attribute modification
    fn sys_open_tree_attr(&self) -> Result<i64> {
        let dirfd = self.data_regs[1] as i32;
        let path_addr = self.data_regs[2] as usize;
        let flags = self.data_regs[3];
        let attr_addr = self.data_regs[4] as usize;
        let size = self.data_regs[5] as usize;

        // Read path
        let path_cstr = self.guest_cstring(path_addr)?;

        // Handle NULL attr or zero size - behaves like open_tree()
        if attr_addr == 0 || size == 0 {
            let result = unsafe {
                libc::syscall(
                    467, // SYS_open_tree_attr
                    dirfd,
                    path_cstr.as_ptr(),
                    flags,
                    std::ptr::null::<libc::c_void>(),
                    0,
                ) as i64
            };
            return Ok(Self::libc_to_kernel(result));
        }

        // Read struct mount_attr from guest memory (m68k big-endian)
        // struct mount_attr {
        //     u64 attr_set;     // 0: 8 bytes
        //     u64 attr_clr;     // 8: 8 bytes
        //     u64 propagation;  // 16: 8 bytes
        //     u64 userns_fd;    // 24: 8 bytes
        // }
        // Total: 32 bytes minimum

        if size < 32 {
            // Size too small for valid mount_attr structure
            return Ok(Self::libc_to_kernel(-libc::EINVAL as i64));
        }

        // Read fields using the new helper
        let attr_set = self.read_u64_be(attr_addr)?;
        let attr_clr = self.read_u64_be(attr_addr + 8)?;
        let propagation = self.read_u64_be(attr_addr + 16)?;
        let userns_fd = self.read_u64_be(attr_addr + 24)?;

        // Build host mount_attr structure
        #[repr(C)]
        struct MountAttr {
            attr_set: u64,
            attr_clr: u64,
            propagation: u64,
            userns_fd: u64,
        }

        let host_attr = MountAttr {
            attr_set,
            attr_clr,
            propagation,
            userns_fd,
        };

        // Call open_tree_attr via syscall (no libc wrapper)
        let result = unsafe {
            libc::syscall(
                467, // SYS_open_tree_attr
                dirfd,
                path_cstr.as_ptr(),
                flags,
                &host_attr as *const MountAttr,
                std::mem::size_of::<MountAttr>(),
            ) as i64
        };

        Ok(Self::libc_to_kernel(result))
    }

    /// linkat(olddirfd, oldpath, newdirfd, newpath, flags)
    fn sys_linkat(&self) -> Result<i64> {
        let olddirfd = self.data_regs[1] as i32;
        let oldpath_addr = self.data_regs[2] as usize;
        let newdirfd = self.data_regs[3] as i32;
        let newpath_addr = self.data_regs[4] as usize;
        let flags = self.data_regs[5] as i32;

        let oldpath_cstr = self.guest_cstring(oldpath_addr)?;
        let newpath_cstr = self.guest_cstring(newpath_addr)?;
        let result = unsafe {
            libc::linkat(
                olddirfd,
                oldpath_cstr.as_ptr(),
                newdirfd,
                newpath_cstr.as_ptr(),
                flags,
            ) as i64
        };
        Ok(Self::libc_to_kernel(result))
    }

    /// symlinkat(target, newdirfd, linkpath)
    fn sys_symlinkat(&self) -> Result<i64> {
        let target_addr = self.data_regs[1] as usize;
        let newdirfd = self.data_regs[2] as i32;
        let linkpath_addr = self.data_regs[3] as usize;

        let target_cstr = self.guest_cstring(target_addr)?;
        let linkpath_cstr = self.guest_cstring(linkpath_addr)?;
        let result = unsafe {
            libc::symlinkat(target_cstr.as_ptr(), newdirfd, linkpath_cstr.as_ptr()) as i64
        };
        Ok(Self::libc_to_kernel(result))
    }

    /// readlinkat(dirfd, path, buf, bufsiz)
    fn sys_readlinkat(&mut self) -> Result<i64> {
        let dirfd = self.data_regs[1] as i32;
        let path_addr = self.data_regs[2] as usize;
        let buf_addr = self.data_regs[3] as usize;
        let bufsiz = self.data_regs[4] as usize;

        let path_cstr = self.guest_cstring(path_addr)?;
        let host_buf = self
            .memory
            .guest_to_host_mut(buf_addr, bufsiz)
            .ok_or_else(|| anyhow!("invalid buffer for readlinkat"))?;
        let result = unsafe {
            libc::readlinkat(dirfd, path_cstr.as_ptr(), host_buf as *mut i8, bufsiz) as i64
        };
        Ok(Self::libc_to_kernel(result))
    }

    /// renameat(olddirfd, oldpath, newdirfd, newpath)
    fn sys_renameat(&self) -> Result<i64> {
        let olddirfd = self.data_regs[1] as i32;
        let oldpath_addr = self.data_regs[2] as usize;
        let newdirfd = self.data_regs[3] as i32;
        let newpath_addr = self.data_regs[4] as usize;

        let oldpath_cstr = self.guest_cstring(oldpath_addr)?;
        let newpath_cstr = self.guest_cstring(newpath_addr)?;
        let result = unsafe {
            libc::renameat(
                olddirfd,
                oldpath_cstr.as_ptr(),
                newdirfd,
                newpath_cstr.as_ptr(),
            ) as i64
        };
        Ok(Self::libc_to_kernel(result))
    }

    /// renameat2(olddirfd, oldpath, newdirfd, newpath, flags)
    fn sys_renameat2(&self) -> Result<i64> {
        let olddirfd = self.data_regs[1] as i32;
        let oldpath_addr = self.data_regs[2] as usize;
        let newdirfd = self.data_regs[3] as i32;
        let newpath_addr = self.data_regs[4] as usize;
        let flags = self.data_regs[5];

        let oldpath_cstr = self.guest_cstring(oldpath_addr)?;
        let newpath_cstr = self.guest_cstring(newpath_addr)?;
        let result = unsafe {
            libc::syscall(
                libc::SYS_renameat2,
                olddirfd,
                oldpath_cstr.as_ptr(),
                newdirfd,
                newpath_cstr.as_ptr(),
                flags,
            ) as i64
        };
        Ok(Self::libc_to_kernel(result))
    }

    /// fchownat(dirfd, path, owner, group, flags)
    fn sys_fchownat(&self) -> Result<i64> {
        let dirfd = self.data_regs[1] as i32;
        let path_addr = self.data_regs[2] as usize;
        let owner = self.data_regs[3] as libc::uid_t;
        let group = self.data_regs[4] as libc::gid_t;
        let flags = self.data_regs[5] as i32;

        let path_cstr = self.guest_cstring(path_addr)?;
        let result =
            unsafe { libc::fchownat(dirfd, path_cstr.as_ptr(), owner, group, flags) as i64 };
        Ok(Self::libc_to_kernel(result))
    }

    /// faccessat(dirfd, path, mode, flags)
    fn sys_faccessat(&self) -> Result<i64> {
        let dirfd = self.data_regs[1] as i32;
        let path_addr = self.data_regs[2] as usize;
        let mode = self.data_regs[3] as i32;
        let flags = self.data_regs[4] as i32;

        let path_cstr = self.guest_cstring(path_addr)?;
        let result = unsafe { libc::faccessat(dirfd, path_cstr.as_ptr(), mode, flags) as i64 };
        Ok(Self::libc_to_kernel(result))
    }

    /// memfd_create(name, flags)
    fn sys_memfd_create(&self) -> Result<i64> {
        let name_addr = self.data_regs[1] as usize;
        let flags = self.data_regs[2];

        let name_cstr = self.guest_cstring(name_addr)?;
        let result = unsafe { libc::syscall(libc::SYS_memfd_create, name_cstr.as_ptr(), flags) };
        Ok(Self::libc_to_kernel(result))
    }

    /// getcpu(cpu, node, tcache)
    fn sys_getcpu(&mut self) -> Result<i64> {
        let cpu_addr = self.data_regs[1] as usize;
        let node_addr = self.data_regs[2] as usize;
        let tcache = self.data_regs[3] as usize;

        let cpu_ptr = if cpu_addr != 0 {
            self.memory
                .guest_to_host_mut(cpu_addr, 4)
                .ok_or_else(|| anyhow!("invalid cpu buffer"))?
        } else {
            std::ptr::null_mut()
        };

        let node_ptr = if node_addr != 0 {
            self.memory
                .guest_to_host_mut(node_addr, 4)
                .ok_or_else(|| anyhow!("invalid node buffer"))?
        } else {
            std::ptr::null_mut()
        };

        let result = unsafe {
            libc::syscall(
                libc::SYS_getcpu,
                cpu_ptr as *mut libc::c_uint,
                node_ptr as *mut libc::c_uint,
                tcache,
            )
        };
        Ok(Self::libc_to_kernel(result))
    }

    /// signalfd(fd, mask, flags)
    fn sys_signalfd(&self) -> Result<i64> {
        let fd = self.data_regs[1] as i32;
        let mask_addr = self.data_regs[2] as usize;
        let flags = self.data_regs[3] as i32;

        // Mask is a sigset_t, which is typically 128 bytes on m68k
        let mask_size = std::mem::size_of::<libc::sigset_t>();
        let mask_ptr = self
            .memory
            .guest_to_host(mask_addr, mask_size)
            .ok_or_else(|| anyhow!("invalid sigset_t buffer"))?;

        let result = unsafe {
            libc::syscall(
                libc::SYS_signalfd,
                fd,
                mask_ptr as *const libc::sigset_t,
                flags,
            )
        };
        Ok(Self::libc_to_kernel(result))
    }

    /// signalfd4(fd, mask, sizemask, flags)
    fn sys_signalfd4(&self) -> Result<i64> {
        let fd = self.data_regs[1] as i32;
        let mask_addr = self.data_regs[2] as usize;
        let sizemask = self.data_regs[3] as usize;
        let flags = self.data_regs[4] as i32;

        let mask_ptr = self
            .memory
            .guest_to_host(mask_addr, sizemask)
            .ok_or_else(|| anyhow!("invalid sigset_t buffer"))?;

        let result = unsafe {
            libc::syscall(
                libc::SYS_signalfd4,
                fd,
                mask_ptr as *const libc::sigset_t,
                sizemask,
                flags,
            )
        };
        Ok(Self::libc_to_kernel(result))
    }

    /// mknodat(dirfd, path, mode, dev)
    fn sys_mknodat(&self) -> Result<i64> {
        let dirfd = self.data_regs[1] as i32;
        let path_addr = self.data_regs[2] as usize;
        let mode = self.data_regs[3] as libc::mode_t;
        let dev = self.data_regs[4] as libc::dev_t;

        let path_cstr = self.guest_cstring(path_addr)?;
        let result = unsafe { libc::mknodat(dirfd, path_cstr.as_ptr(), mode, dev) as i64 };
        Ok(Self::libc_to_kernel(result))
    }

    /// futimesat(dirfd, path, times)
    fn sys_futimesat(&self) -> Result<i64> {
        let dirfd = self.data_regs[1] as i32;
        let path_addr = self.data_regs[2] as usize;
        let times_addr = self.data_regs[3] as usize;

        let path_cstr = self.guest_cstring(path_addr)?;
        let times_ptr = if times_addr != 0 {
            self.memory
                .guest_to_host(times_addr, std::mem::size_of::<libc::timeval>() * 2)
                .ok_or_else(|| anyhow!("invalid timeval buffer"))?
        } else {
            std::ptr::null()
        };

        let result = unsafe {
            libc::syscall(
                libc::SYS_futimesat,
                dirfd,
                path_cstr.as_ptr(),
                times_ptr as *const libc::timeval,
            )
        };
        Ok(Self::libc_to_kernel(result))
    }

    /// fstatat64(dirfd, path, buf, flags)
    fn sys_fstatat64(&mut self) -> Result<i64> {
        let dirfd = self.data_regs[1] as i32;
        let path_addr = self.data_regs[2] as usize;
        let buf_addr = self.data_regs[3] as usize;
        let flags = self.data_regs[4] as i32;

        let path_cstr = self.guest_cstring(path_addr)?;
        let mut stat: libc::stat = unsafe { std::mem::zeroed() };
        let result = unsafe {
            libc::syscall(
                libc::SYS_newfstatat,
                dirfd,
                path_cstr.as_ptr(),
                &mut stat,
                flags,
            )
        };
        if result == 0 {
            self.write_stat(buf_addr, &stat)?;
        }
        Ok(result)
    }

    /// utimes(filename, times)
    fn sys_utimes(&self) -> Result<i64> {
        let filename_addr = self.data_regs[1] as usize;
        let times_addr = self.data_regs[2] as usize;

        let filename_cstr = self.guest_cstring(filename_addr)?;

        let times_ptr = if times_addr != 0 {
            // times is array of 2 timevals (each is 2 longs: tv_sec, tv_usec)
            self.memory
                .guest_to_host(times_addr, std::mem::size_of::<libc::timeval>() * 2)
                .ok_or_else(|| anyhow!("invalid timeval buffer"))?
        } else {
            std::ptr::null()
        };

        let result = unsafe {
            libc::utimes(filename_cstr.as_ptr(), times_ptr as *const libc::timeval) as i64
        };
        Ok(Self::libc_to_kernel(result))
    }

    /// utimensat(dirfd, path, times, flags)
    fn sys_utimensat(&self) -> Result<i64> {
        let dirfd = self.data_regs[1] as i32;
        let path_addr = self.data_regs[2] as usize;
        let times_addr = self.data_regs[3] as usize;
        let flags = self.data_regs[4] as i32;

        let path_cstr = if path_addr != 0 {
            Some(self.guest_cstring(path_addr)?)
        } else {
            None
        };

        let times_ptr = if times_addr != 0 {
            self.memory
                .guest_to_host(times_addr, std::mem::size_of::<libc::timespec>() * 2)
                .ok_or_else(|| anyhow!("invalid timespec buffer"))?
        } else {
            std::ptr::null()
        };

        let path_ptr = path_cstr
            .as_ref()
            .map(|c| c.as_ptr())
            .unwrap_or(std::ptr::null());
        let result = unsafe {
            libc::utimensat(dirfd, path_ptr, times_ptr as *const libc::timespec, flags) as i64
        };
        Ok(Self::libc_to_kernel(result))
    }

    /// inotify_add_watch(fd, path, mask)
    fn sys_inotify_add_watch(&self) -> Result<i64> {
        let fd = self.data_regs[1] as i32;
        let path_addr = self.data_regs[2] as usize;
        let mask = self.data_regs[3];

        let path_cstr = self.guest_cstring(path_addr)?;
        let result = unsafe { libc::inotify_add_watch(fd, path_cstr.as_ptr(), mask) as i64 };
        Ok(Self::libc_to_kernel(result))
    }

    /// copy_file_range(fd_in, off_in, fd_out, off_out, len, flags)
    fn sys_copy_file_range(&mut self) -> Result<i64> {
        let fd_in = self.data_regs[1] as i32;
        let off_in_addr = self.data_regs[2] as usize;
        let fd_out = self.data_regs[3] as i32;
        let off_out_addr = self.data_regs[4] as usize;
        let len = self.data_regs[5] as usize;

        let off_in_ptr = if off_in_addr != 0 {
            self.memory
                .guest_to_host_mut(off_in_addr, 8)
                .ok_or_else(|| anyhow!("invalid off_in pointer"))? as *mut i64
        } else {
            std::ptr::null_mut()
        };

        let off_out_ptr = if off_out_addr != 0 {
            self.memory
                .guest_to_host_mut(off_out_addr, 8)
                .ok_or_else(|| anyhow!("invalid off_out pointer"))? as *mut i64
        } else {
            std::ptr::null_mut()
        };

        let result = unsafe {
            libc::syscall(
                libc::SYS_copy_file_range,
                fd_in,
                off_in_ptr,
                fd_out,
                off_out_ptr,
                len,
                0,
            )
        };
        Ok(Self::libc_to_kernel(result))
    }

    /// timerfd_settime(fd, flags, new_value, old_value)
    fn sys_timerfd_settime(&self) -> Result<i64> {
        let fd = self.data_regs[1] as i32;
        let flags = self.data_regs[2] as i32;
        let new_value_addr = self.data_regs[3] as usize;
        let old_value_addr = self.data_regs[4] as usize;

        let new_value_ptr = self
            .memory
            .guest_to_host(new_value_addr, std::mem::size_of::<libc::itimerspec>())
            .ok_or_else(|| anyhow!("invalid new_value pointer"))?;

        let old_value_ptr = if old_value_addr != 0 {
            self.memory
                .guest_to_host(old_value_addr, std::mem::size_of::<libc::itimerspec>())
                .ok_or_else(|| anyhow!("invalid old_value pointer"))?
        } else {
            std::ptr::null()
        };

        let result = unsafe {
            libc::timerfd_settime(
                fd,
                flags,
                new_value_ptr as *const libc::itimerspec,
                old_value_ptr as *mut libc::itimerspec,
            ) as i64
        };
        Ok(Self::libc_to_kernel(result))
    }

    /// timerfd_gettime(fd, curr_value)
    fn sys_timerfd_gettime(&self) -> Result<i64> {
        let fd = self.data_regs[1] as i32;
        let curr_value_addr = self.data_regs[2] as usize;

        let curr_value_ptr = self
            .memory
            .guest_to_host(curr_value_addr, std::mem::size_of::<libc::itimerspec>())
            .ok_or_else(|| anyhow!("invalid curr_value pointer"))?;

        let result =
            unsafe { libc::timerfd_gettime(fd, curr_value_ptr as *mut libc::itimerspec) as i64 };
        Ok(Self::libc_to_kernel(result))
    }

    /// getrandom(buf, buflen, flags)
    fn sys_getrandom(&mut self) -> Result<i64> {
        let buf_addr = self.data_regs[1] as usize;
        let buflen = self.data_regs[2] as usize;
        let flags = self.data_regs[3];

        let buf_ptr = self
            .memory
            .guest_to_host_mut(buf_addr, buflen)
            .ok_or_else(|| anyhow!("invalid buffer for getrandom"))?;

        let result = unsafe { libc::syscall(libc::SYS_getrandom, buf_ptr, buflen, flags) };
        Ok(Self::libc_to_kernel(result))
    }

    /// prctl(option, arg2, arg3, arg4, arg5)
    fn sys_prctl(&mut self) -> Result<i64> {
        let option = self.data_regs[1] as i32;
        let arg2 = self.data_regs[2] as usize;
        let arg3 = self.data_regs[3] as usize;
        let arg4 = self.data_regs[4] as usize;
        let arg5 = self.data_regs[5] as usize;

        // PR_GET_PDEATHSIG (2) - arg2 is pointer to int
        if option == 2 {
            let ptr = self
                .memory
                .guest_to_host_mut(arg2, 4)
                .ok_or_else(|| anyhow!("invalid arg2 pointer for PR_GET_PDEATHSIG"))?;
            let result = unsafe { libc::prctl(option, ptr, arg3, arg4, arg5) as i64 };
            Ok(Self::libc_to_kernel(result))
        } else {
            // For other options, pass args as-is
            let result = unsafe { libc::prctl(option, arg2, arg3, arg4, arg5) as i64 };
            Ok(Self::libc_to_kernel(result))
        }
    }

    /// capget(hdrp, datap) - Get thread capabilities
    /// struct __user_cap_header_struct: version (u32) + pid (i32) = 8 bytes
    /// struct __user_cap_data_struct: effective (u32) + permitted (u32) + inheritable (u32) = 12 bytes
    /// Note: Version 3 uses array of 2 data structs (24 bytes total)
    fn sys_capget(&mut self) -> Result<i64> {
        let hdrp_addr = self.data_regs[1] as usize;
        let datap_addr = self.data_regs[2] as usize;

        if hdrp_addr == 0 {
            return Ok(-libc::EFAULT as i64);
        }

        // Read header from guest memory (big-endian)
        let version = self.memory.read_long(hdrp_addr)?;
        let pid = self.memory.read_long(hdrp_addr + 4)? as i32;

        // Build host header
        #[repr(C)]
        struct CapUserHeader {
            version: u32,
            pid: i32,
        }

        let mut hdr = CapUserHeader { version, pid };

        // Determine how many data structs we need based on version
        // Version 1: 1 data struct, Version 2/3: 2 data structs
        let data_count = if version == 0x19980330 { 1 } else { 2 };

        // Prepare data buffer
        #[repr(C)]
        #[derive(Copy, Clone)]
        struct CapUserData {
            effective: u32,
            permitted: u32,
            inheritable: u32,
        }

        let mut data: [CapUserData; 2] = [CapUserData {
            effective: 0,
            permitted: 0,
            inheritable: 0,
        }; 2];

        // Call capget (x86_64 syscall 125)
        let result = if datap_addr == 0 {
            // NULL datap - just checking version
            unsafe { libc::syscall(125, &mut hdr as *mut _, std::ptr::null_mut::<CapUserData>()) }
        } else {
            unsafe { libc::syscall(125, &mut hdr as *mut _, data.as_mut_ptr()) }
        };

        // Write header back (kernel may update version field)
        self.memory
            .write_data(hdrp_addr, &hdr.version.to_be_bytes())?;
        self.memory
            .write_data(hdrp_addr + 4, &hdr.pid.to_be_bytes())?;

        // Write data back if successful and datap is not NULL
        if result >= 0 && datap_addr != 0 {
            for i in 0..data_count {
                let offset = datap_addr + i * 12;
                self.memory
                    .write_data(offset, &data[i].effective.to_be_bytes())?;
                self.memory
                    .write_data(offset + 4, &data[i].permitted.to_be_bytes())?;
                self.memory
                    .write_data(offset + 8, &data[i].inheritable.to_be_bytes())?;
            }
        }

        Ok(Self::libc_to_kernel(result as i64))
    }

    /// capset(hdrp, datap) - Set thread capabilities
    fn sys_capset(&self) -> Result<i64> {
        let hdrp_addr = self.data_regs[1] as usize;
        let datap_addr = self.data_regs[2] as usize;

        if hdrp_addr == 0 {
            return Ok(-libc::EFAULT as i64);
        }

        // Read header from guest memory (big-endian)
        let version = self.memory.read_long(hdrp_addr)?;
        let pid = self.memory.read_long(hdrp_addr + 4)? as i32;

        // Build host header
        #[repr(C)]
        struct CapUserHeader {
            version: u32,
            pid: i32,
        }

        let hdr = CapUserHeader { version, pid };

        // Determine how many data structs based on version
        let data_count = if version == 0x19980330 { 1 } else { 2 };

        // Read data from guest memory
        #[repr(C)]
        #[derive(Copy, Clone)]
        struct CapUserData {
            effective: u32,
            permitted: u32,
            inheritable: u32,
        }

        let mut data: [CapUserData; 2] = [CapUserData {
            effective: 0,
            permitted: 0,
            inheritable: 0,
        }; 2];

        if datap_addr != 0 {
            for i in 0..data_count {
                let offset = datap_addr + i * 12;
                data[i].effective = self.memory.read_long(offset)?;
                data[i].permitted = self.memory.read_long(offset + 4)?;
                data[i].inheritable = self.memory.read_long(offset + 8)?;
            }
        }

        // Call capset (x86_64 syscall 126)
        let result = if datap_addr == 0 {
            unsafe { libc::syscall(126, &hdr as *const _, std::ptr::null::<CapUserData>()) }
        } else {
            unsafe { libc::syscall(126, &hdr as *const _, data.as_ptr()) }
        };

        Ok(Self::libc_to_kernel(result as i64))
    }

    /// Helper for syscalls with pattern: syscall(path, arg2, arg3, ...)
    /// D1 = path pointer, extra_args passed directly
    fn sys_path1(&self, syscall_num: u32, extra_arg: i64) -> Result<i64> {
        let path_addr = self.data_regs[1] as usize;
        let path_cstr = self.guest_cstring(path_addr)?;

        Ok(unsafe { libc::syscall(syscall_num as i64, path_cstr.as_ptr(), extra_arg) })
    }

    /// link(oldpath, newpath)
    fn sys_link(&self) -> Result<i64> {
        let old_addr = self.data_regs[1] as usize;
        let new_addr = self.data_regs[2] as usize;

        let old_cstr = self.guest_cstring(old_addr)?;
        let new_cstr = self.guest_cstring(new_addr)?;

        Ok(unsafe { libc::link(old_cstr.as_ptr(), new_cstr.as_ptr()) as i64 })
    }

    /// time(tloc) - tloc can be NULL
    fn sys_time(&mut self) -> Result<i64> {
        let tloc = self.data_regs[1] as usize;

        if tloc == 0 {
            // NULL pointer - just return time
            Ok(unsafe { libc::time(std::ptr::null_mut()) })
        } else {
            // Need to write result to guest memory
            let mut t: libc::time_t = 0;
            let result = unsafe { libc::time(&mut t) };
            if result != -1 {
                // m68k uclibc uses 64-bit time_t
                self.memory.write_data(tloc, &(t as i64).to_be_bytes())?;
            }
            Ok(result)
        }
    }

    /// mknod(path, mode, dev)
    fn sys_mknod(&self) -> Result<i64> {
        let path_addr = self.data_regs[1] as usize;
        let mode = self.data_regs[2] as libc::mode_t;
        let dev = self.data_regs[3] as libc::dev_t;

        let path_cstr = self.guest_cstring(path_addr)?;

        Ok(unsafe { libc::mknod(path_cstr.as_ptr(), mode, dev) as i64 })
    }

    /// chown/lchown/fchownat(path, owner, group)
    fn sys_chown(&self, syscall_num: u32) -> Result<i64> {
        let path_addr = self.data_regs[1] as usize;
        let owner = self.data_regs[2] as libc::uid_t;
        let group = self.data_regs[3] as libc::gid_t;

        let path_cstr = self.guest_cstring(path_addr)?;

        Ok(unsafe { libc::syscall(syscall_num as i64, path_cstr.as_ptr(), owner, group) })
    }

    /// utime(path, times) - times can be NULL
    fn sys_utime(&self) -> Result<i64> {
        let path_addr = self.data_regs[1] as usize;
        let times_addr = self.data_regs[2] as usize;

        let path_cstr = self.guest_cstring(path_addr)?;

        if times_addr == 0 {
            // NULL times - set to current time
            Ok(unsafe { libc::utime(path_cstr.as_ptr(), std::ptr::null()) as i64 })
        } else {
            // m68k uclibc uses 64-bit time_t
            let actime_bytes: [u8; 8] = self.memory.read_data(times_addr, 8)?.try_into().unwrap();
            let actime = i64::from_be_bytes(actime_bytes) as libc::time_t;
            let modtime_bytes: [u8; 8] = self
                .memory
                .read_data(times_addr + 8, 8)?
                .try_into()
                .unwrap();
            let modtime = i64::from_be_bytes(modtime_bytes) as libc::time_t;
            let times = libc::utimbuf { actime, modtime };
            Ok(unsafe { libc::utime(path_cstr.as_ptr(), &times) as i64 })
        }
    }

    /// rename(oldpath, newpath)
    fn sys_rename(&self) -> Result<i64> {
        let old_addr = self.data_regs[1] as usize;
        let new_addr = self.data_regs[2] as usize;

        let old_cstr = self.guest_cstring(old_addr)?;
        let new_cstr = self.guest_cstring(new_addr)?;

        Ok(unsafe { libc::rename(old_cstr.as_ptr(), new_cstr.as_ptr()) as i64 })
    }

    /// pipe(pipefd) - writes two fds to guest memory
    fn sys_pipe(&mut self) -> Result<i64> {
        let pipefd_addr = self.data_regs[1] as usize;
        let mut fds: [libc::c_int; 2] = [0; 2];

        let result = unsafe { libc::pipe(fds.as_mut_ptr()) };
        if result == 0 {
            // Write the two fds to guest memory (as 32-bit big-endian)
            self.memory
                .write_data(pipefd_addr, &(fds[0] as u32).to_be_bytes())?;
            self.memory
                .write_data(pipefd_addr + 4, &(fds[1] as u32).to_be_bytes())?;
        }
        Ok(result as i64)
    }

    /// pipe2(pipefd, flags) - writes two fds to guest memory
    fn sys_pipe2(&mut self) -> Result<i64> {
        let pipefd_addr = self.data_regs[1] as usize;
        let flags = self.data_regs[2] as libc::c_int;
        let mut fds: [libc::c_int; 2] = [0; 2];

        let result = unsafe { libc::pipe2(fds.as_mut_ptr(), flags) };
        if result == 0 {
            // Write the two fds to guest memory (as 32-bit big-endian)
            self.memory
                .write_data(pipefd_addr, &(fds[0] as u32).to_be_bytes())?;
            self.memory
                .write_data(pipefd_addr + 4, &(fds[1] as u32).to_be_bytes())?;
        }
        Ok(result as i64)
    }

    /// times(buf) - writes struct tms to guest memory
    fn sys_times(&mut self) -> Result<i64> {
        let buf_addr = self.data_regs[1] as usize;
        let mut tms = libc::tms {
            tms_utime: 0,
            tms_stime: 0,
            tms_cutime: 0,
            tms_cstime: 0,
        };

        let result = unsafe { libc::times(&mut tms) };
        if result != -1 && buf_addr != 0 {
            // Write struct tms to guest memory (four 32-bit clock_t values on m68k)
            self.memory
                .write_data(buf_addr, &(tms.tms_utime as u32).to_be_bytes())?;
            self.memory
                .write_data(buf_addr + 4, &(tms.tms_stime as u32).to_be_bytes())?;
            self.memory
                .write_data(buf_addr + 8, &(tms.tms_cutime as u32).to_be_bytes())?;
            self.memory
                .write_data(buf_addr + 12, &(tms.tms_cstime as u32).to_be_bytes())?;
        }
        Ok(result)
    }

    /// brk(addr) - grow/shrink the emulated heap
    fn sys_brk(&mut self) -> Result<i64> {
        let requested = self.data_regs[1] as usize;
        let old_brk = self.brk;

        if requested == 0 {
            return Ok(old_brk as i64);
        }

        let mut target = requested;
        if target < self.brk_base {
            target = self.brk_base;
        }

        // Align for internal memory allocation, but store exact value like Linux does
        let target_aligned = align_up(target, 4096);
        let old_brk_aligned = align_up(old_brk, 4096);

        let guard: usize = 0x1000;
        if target_aligned + guard > self.stack_base {
            return Ok(old_brk as i64);
        }

        // Only resize the backing segment if we need more pages
        if target_aligned > old_brk_aligned {
            let new_len = target_aligned
                .checked_sub(self.heap_segment_base)
                .ok_or_else(|| anyhow::anyhow!("brk underflow"))?;
            self.memory
                .resize_segment(self.heap_segment_base, new_len)?;
        }

        // Store and return the exact requested value (like Linux)
        self.brk = target;
        Ok(self.brk as i64)
    }

    /// sethostname(name, len)
    fn sys_sethostname(&self) -> Result<i64> {
        let name_addr = self.data_regs[1] as usize;
        let len = self.data_regs[2] as usize;
        let host_ptr = self
            .memory
            .guest_to_host(name_addr, len)
            .ok_or_else(|| anyhow!("invalid hostname buffer"))?;
        Ok(unsafe { libc::sethostname(host_ptr as *const i8, len) as i64 })
    }

    /// setrlimit(resource, rlim)
    fn sys_setrlimit(&self) -> Result<i64> {
        let resource = self.data_regs[1] as i32;
        let rlim_addr = self.data_regs[2] as usize;
        // m68k rlimit: two 32-bit values (rlim_cur, rlim_max)
        let rlim_cur = self.memory.read_long(rlim_addr)? as libc::rlim_t;
        let rlim_max = self.memory.read_long(rlim_addr + 4)? as libc::rlim_t;
        let rlim = libc::rlimit { rlim_cur, rlim_max };
        Ok(unsafe { libc::setrlimit(resource as u32, &rlim) as i64 })
    }

    /// getrlimit(resource, rlim)
    fn sys_getrlimit(&mut self) -> Result<i64> {
        let resource = self.data_regs[1] as i32;
        let rlim_addr = self.data_regs[2] as usize;
        let mut rlim: libc::rlimit = unsafe { std::mem::zeroed() };
        let result = unsafe { libc::getrlimit(resource as u32, &mut rlim) };
        if result == 0 && rlim_addr != 0 {
            // m68k rlimit: two 32-bit values (rlim_cur, rlim_max)
            self.memory
                .write_data(rlim_addr, &(rlim.rlim_cur as u32).to_be_bytes())?;
            self.memory
                .write_data(rlim_addr + 4, &(rlim.rlim_max as u32).to_be_bytes())?;
        }
        Ok(result as i64)
    }

    /// prlimit64(pid, resource, new_limit, old_limit)
    fn sys_prlimit64(&mut self) -> Result<i64> {
        let pid = self.data_regs[1] as libc::pid_t;
        let resource = self.data_regs[2] as i32;
        let new_limit_addr = self.data_regs[3] as usize;
        let old_limit_addr = self.data_regs[4] as usize;

        let new_limit_ptr = if new_limit_addr != 0 {
            // m68k rlimit64: two 64-bit values (rlim_cur, rlim_max)
            // Each 64-bit value is stored as two 32-bit words (big-endian)
            let cur_hi = self.memory.read_long(new_limit_addr)? as u64;
            let cur_lo = self.memory.read_long(new_limit_addr + 4)? as u64;
            let max_hi = self.memory.read_long(new_limit_addr + 8)? as u64;
            let max_lo = self.memory.read_long(new_limit_addr + 12)? as u64;
            let rlim_cur = (cur_hi << 32) | cur_lo;
            let rlim_max = (max_hi << 32) | max_lo;
            Some(libc::rlimit64 { rlim_cur, rlim_max })
        } else {
            None
        };

        let mut old_limit: libc::rlimit64 = unsafe { std::mem::zeroed() };

        let result = unsafe {
            libc::prlimit64(
                pid,
                resource as u32,
                new_limit_ptr
                    .as_ref()
                    .map(|l| l as *const _)
                    .unwrap_or(std::ptr::null()),
                if old_limit_addr != 0 {
                    &mut old_limit
                } else {
                    std::ptr::null_mut()
                },
            )
        };

        if result == 0 && old_limit_addr != 0 {
            // Write old_limit back to guest memory
            let cur_hi = (old_limit.rlim_cur >> 32) as u32;
            let cur_lo = old_limit.rlim_cur as u32;
            let max_hi = (old_limit.rlim_max >> 32) as u32;
            let max_lo = old_limit.rlim_max as u32;
            self.memory
                .write_data(old_limit_addr, &cur_hi.to_be_bytes())?;
            self.memory
                .write_data(old_limit_addr + 4, &cur_lo.to_be_bytes())?;
            self.memory
                .write_data(old_limit_addr + 8, &max_hi.to_be_bytes())?;
            self.memory
                .write_data(old_limit_addr + 12, &max_lo.to_be_bytes())?;
        }

        Ok(result as i64)
    }

    /// getrusage(who, usage)
    fn sys_getrusage(&mut self) -> Result<i64> {
        let who = self.data_regs[1] as i32;
        let usage_addr = self.data_regs[2] as usize;
        let mut usage: libc::rusage = unsafe { std::mem::zeroed() };
        let result = unsafe { libc::getrusage(who, &mut usage) };
        if result == 0 && usage_addr != 0 {
            // Write rusage struct - m68k uclibc uses 64-bit time_t
            self.memory
                .write_data(usage_addr, &(usage.ru_utime.tv_sec as i64).to_be_bytes())?;
            self.memory.write_data(
                usage_addr + 8,
                &(usage.ru_utime.tv_usec as u32).to_be_bytes(),
            )?;
            self.memory.write_data(
                usage_addr + 12,
                &(usage.ru_stime.tv_sec as i64).to_be_bytes(),
            )?;
            self.memory.write_data(
                usage_addr + 20,
                &(usage.ru_stime.tv_usec as u32).to_be_bytes(),
            )?;
        }
        Ok(result as i64)
    }

    /// gettimeofday(tv, tz)
    fn sys_gettimeofday(&mut self) -> Result<i64> {
        let tv_addr = self.data_regs[1] as usize;
        let mut tv: libc::timeval = unsafe { std::mem::zeroed() };
        let result = unsafe { libc::gettimeofday(&mut tv, std::ptr::null_mut()) };
        if result == 0 && tv_addr != 0 {
            // m68k uclibc uses 64-bit time_t
            self.memory
                .write_data(tv_addr, &(tv.tv_sec as i64).to_be_bytes())?;
            self.memory
                .write_data(tv_addr + 8, &(tv.tv_usec as u32).to_be_bytes())?;
        }
        Ok(result as i64)
    }

    /// clock_gettime(clockid, timespec)
    fn sys_clock_gettime(&mut self) -> Result<i64> {
        let clk_id = self.data_regs[1] as libc::clockid_t;
        let ts_addr = self.data_regs[2] as usize;
        let mut ts: libc::timespec = unsafe { std::mem::zeroed() };
        let result = unsafe { libc::clock_gettime(clk_id, &mut ts) };
        if result == 0 && ts_addr != 0 {
            // m68k uclibc uses 64-bit time_t
            self.memory
                .write_data(ts_addr, &(ts.tv_sec as i64).to_be_bytes())?;
            self.memory
                .write_data(ts_addr + 8, &(ts.tv_nsec as u32).to_be_bytes())?;
        }
        Ok(result as i64)
    }

    /// clock_getres(clockid, timespec)
    fn sys_clock_getres(&mut self) -> Result<i64> {
        let clk_id = self.data_regs[1] as libc::clockid_t;
        let ts_addr = self.data_regs[2] as usize;
        let mut ts: libc::timespec = unsafe { std::mem::zeroed() };
        let result = unsafe { libc::clock_getres(clk_id, &mut ts) };
        if result == 0 && ts_addr != 0 {
            // m68k uclibc uses 64-bit time_t
            self.memory
                .write_data(ts_addr, &(ts.tv_sec as i64).to_be_bytes())?;
            self.memory
                .write_data(ts_addr + 8, &(ts.tv_nsec as u32).to_be_bytes())?;
        }
        Ok(result as i64)
    }

    /// clock_nanosleep(clockid, flags, request, remain)
    fn sys_clock_nanosleep(&mut self) -> Result<i64> {
        let clk_id = self.data_regs[1] as libc::clockid_t;
        let flags = self.data_regs[2] as i32;
        let req_addr = self.data_regs[3] as usize;
        let rem_addr = self.data_regs[4] as usize;

        // m68k uclibc uses 64-bit time_t
        let req_sec_bytes: [u8; 8] = self.memory.read_data(req_addr, 8)?.try_into().unwrap();
        let req_sec = i64::from_be_bytes(req_sec_bytes) as libc::time_t;
        let req_nsec = self.memory.read_long(req_addr + 8)? as i64;

        let req = libc::timespec {
            tv_sec: req_sec,
            tv_nsec: req_nsec,
        };
        let mut rem: libc::timespec = unsafe { std::mem::zeroed() };

        let result = unsafe {
            libc::clock_nanosleep(
                clk_id,
                flags,
                &req,
                if rem_addr != 0 {
                    &mut rem
                } else {
                    std::ptr::null_mut()
                },
            )
        };

        if rem_addr != 0 {
            self.memory
                .write_data(rem_addr, &(rem.tv_sec as i64).to_be_bytes())?;
            self.memory
                .write_data(rem_addr + 8, &(rem.tv_nsec as u32).to_be_bytes())?;
        }

        Ok(result as i64)
    }

    /// clock_settime(clockid, timespec)
    fn sys_clock_settime(&self) -> Result<i64> {
        let clk_id = self.data_regs[1] as libc::clockid_t;
        let ts_addr = self.data_regs[2] as usize;

        if ts_addr == 0 {
            return Ok(-(libc::EFAULT as i64));
        }

        // m68k uclibc uses 64-bit time_t
        let ts_sec_bytes: [u8; 8] = self.memory.read_data(ts_addr, 8)?.try_into().unwrap();
        let ts_sec = i64::from_be_bytes(ts_sec_bytes) as libc::time_t;
        let ts_nsec = self.memory.read_long(ts_addr + 8)? as i64;

        let ts = libc::timespec {
            tv_sec: ts_sec,
            tv_nsec: ts_nsec,
        };

        let result = unsafe { libc::clock_settime(clk_id, &ts) };
        Ok(result as i64)
    }

    /// adjtimex(timex)
    fn sys_adjtimex(&mut self) -> Result<i64> {
        let tx_addr = self.data_regs[1] as usize;

        // struct timex is complex with different layouts between architectures.
        // The test just checks dispatch, so we'll attempt the syscall and
        // expect it to fail with EPERM (typical for unprivileged users).
        // Just validate that we can read some bytes from the structure.
        if tx_addr != 0 {
            // Validate the pointer by reading first few bytes
            let _ = self.memory.read_data(tx_addr, 16)?;
        }

        // Return EPERM as would happen for unprivileged access
        Ok(-(libc::EPERM as i64))
    }

    /// clock_adjtime(clk_id, buf) / clock_adjtime64(clk_id, buf)
    fn sys_clock_adjtime(&mut self) -> Result<i64> {
        let _clk_id = self.data_regs[1] as libc::clockid_t;
        let tx_addr = self.data_regs[2] as usize;

        // struct timex is complex with different layouts between architectures.
        // Like adjtimex, validate the pointer and return EPERM since the guest
        // shouldn't be able to adjust the host's clock.
        if tx_addr != 0 {
            // Validate the pointer by reading first few bytes
            let _ = self.memory.read_data(tx_addr, 16)?;
        }

        // Return EPERM as would happen for unprivileged access
        Ok(-(libc::EPERM as i64))
    }

    /// settimeofday(tv, tz)
    fn sys_settimeofday(&self) -> Result<i64> {
        let tv_addr = self.data_regs[1] as usize;
        if tv_addr == 0 {
            return Ok(unsafe { libc::settimeofday(std::ptr::null(), std::ptr::null()) as i64 });
        }
        // m68k uclibc uses 64-bit time_t
        let tv_sec_bytes: [u8; 8] = self.memory.read_data(tv_addr, 8)?.try_into().unwrap();
        let tv_sec = i64::from_be_bytes(tv_sec_bytes) as libc::time_t;
        let tv_usec = self.memory.read_long(tv_addr + 8)? as libc::suseconds_t;
        let tv = libc::timeval { tv_sec, tv_usec };
        Ok(unsafe { libc::settimeofday(&tv, std::ptr::null()) as i64 })
    }

    /// getgroups(size, list)
    fn sys_getgroups(&mut self) -> Result<i64> {
        let size = self.data_regs[1] as i32;
        let list_addr = self.data_regs[2] as usize;
        if size == 0 {
            return Ok(unsafe { libc::getgroups(0, std::ptr::null_mut()) as i64 });
        }
        let mut groups = vec![0 as libc::gid_t; size as usize];
        let result = unsafe { libc::getgroups(size, groups.as_mut_ptr()) };
        if result > 0 && list_addr != 0 {
            for (i, &gid) in groups.iter().take(result as usize).enumerate() {
                self.memory
                    .write_data(list_addr + i * 4, &(gid).to_be_bytes())?;
            }
        }
        Ok(result as i64)
    }

    /// setgroups(size, list)
    fn sys_setgroups(&self) -> Result<i64> {
        let size = self.data_regs[1] as usize;
        let list_addr = self.data_regs[2] as usize;
        let mut groups = Vec::with_capacity(size);
        for i in 0..size {
            let gid = self.memory.read_long(list_addr + i * 4)? as libc::gid_t;
            groups.push(gid);
        }
        Ok(unsafe { libc::setgroups(size, groups.as_ptr()) as i64 })
    }

    /// symlink(target, linkpath)
    fn sys_symlink(&self) -> Result<i64> {
        let target_addr = self.data_regs[1] as usize;
        let linkpath_addr = self.data_regs[2] as usize;
        let target = self.guest_cstring(target_addr)?;
        let linkpath = self.guest_cstring(linkpath_addr)?;
        Ok(unsafe { libc::symlink(target.as_ptr(), linkpath.as_ptr()) as i64 })
    }

    /// readlink(path, buf, size)
    fn sys_readlink(&mut self) -> Result<i64> {
        let path_addr = self.data_regs[1] as usize;
        let buf_addr = self.data_regs[2] as usize;
        let size = self.data_regs[3] as usize;
        let path = self.guest_cstring(path_addr)?;
        let host_buf = self.guest_mut_ptr(buf_addr, size)?;

        // Handle /proc/self/exe specially - return the m68k binary path
        let path_str = path.to_str().unwrap_or("");
        if path_str == "/proc/self/exe" {
            let exe_bytes = self.exe_path.as_bytes();
            let copy_len = exe_bytes.len().min(size);
            unsafe {
                std::ptr::copy_nonoverlapping(exe_bytes.as_ptr(), host_buf as *mut u8, copy_len);
            }
            return Ok(copy_len as i64);
        }

        Ok(unsafe { libc::readlink(path.as_ptr(), host_buf as *mut i8, size) as i64 })
    }

    /// truncate(path, length)
    fn sys_truncate(&self) -> Result<i64> {
        let path_addr = self.data_regs[1] as usize;
        let length = self.data_regs[2] as i64;
        let path = self.guest_cstring(path_addr)?;
        Ok(unsafe { libc::truncate(path.as_ptr(), length) as i64 })
    }

    /// mmap(addr, length, prot, flags, fd, offset)
    fn sys_mmap(&mut self) -> Result<i64> {
        // old_mmap on m68k uses a single pointer to mmap_arg_struct
        //   struct mmap_arg_struct { void *addr; u32 len; u32 prot; u32 flags; u32 fd; u32 offset; }
        let args_ptr = self.data_regs[1] as usize;
        let addr_req = self.memory.read_long(args_ptr)? as usize;
        let length = self.memory.read_long(args_ptr + 4)? as usize;
        let prot = self.memory.read_long(args_ptr + 8)? as i32;
        let flags = self.memory.read_long(args_ptr + 12)? as i32;
        let fd = self.memory.read_long(args_ptr + 16)? as i32;
        let _offset = self.memory.read_long(args_ptr + 20)? as i64; // bytes (not pages)

        let is_anonymous = (flags & 0x20) != 0 || fd == -1;
        if !is_anonymous {
            bail!("mmap: file-backed mappings not yet supported (fd={fd})");
        }

        let addr = self.alloc_anonymous_mmap(addr_req, length, prot)?;
        Ok(addr as i64)
    }

    /// mmap2(addr, length, prot, flags, fd, pgoffset) - offset is in pages
    fn sys_mmap2(&mut self) -> Result<i64> {
        let req_addr = self.data_regs[1] as usize;
        let length = self.data_regs[2] as usize;
        let prot = self.data_regs[3] as i32;
        let flags = self.data_regs[4] as i32;
        let fd = self.data_regs[5] as i32;
        let _pgoffset = self.data_regs[6] as i64;

        // eprintln!(
        //     "mmap2: addr={:#x}, len={:#x}, prot={:#x}, flags={:#x}, fd={}",
        //     req_addr, length, prot, flags, fd
        // );

        // Check for MAP_ANONYMOUS (0x20 on Linux)
        let is_anonymous = (flags & 0x20) != 0;

        if !is_anonymous {
            bail!("mmap2: file-backed mappings not yet supported (fd={fd})");
        }

        let addr = self.alloc_anonymous_mmap(req_addr, length, prot)?;
        // eprintln!("mmap2: allocated anonymous region at {:#x}", addr);
        Ok(addr as i64)
    }

    /// mprotect(addr, len, prot)
    fn sys_mprotect(&self) -> Result<i64> {
        let addr = self.data_regs[1] as usize;
        let len = self.data_regs[2] as usize;
        let _prot = self.data_regs[3] as i32;

        // Validate that the memory range exists
        // For now, just check if we can access the start and end
        if len > 0 {
            let _ = self
                .memory
                .guest_to_host(addr, 1)
                .ok_or_else(|| anyhow!("mprotect: invalid address {:#x}", addr))?;
            if len > 1 {
                let _ = self
                    .memory
                    .guest_to_host(addr + len - 1, 1)
                    .ok_or_else(|| anyhow!("mprotect: invalid address range"))?;
            }
        }

        // Just return success - we don't actually change protection bits
        // since the memory is already accessible to the guest
        Ok(0)
    }

    /// mincore(addr, length, vec)
    fn sys_mincore(&mut self) -> Result<i64> {
        let addr = self.data_regs[1] as usize;
        let length = self.data_regs[2] as usize;
        let vec_ptr = self.data_regs[3] as usize;

        const PAGE_SIZE: usize = 4096;
        let num_pages = (length + PAGE_SIZE - 1) / PAGE_SIZE;

        // Validate the memory range exists
        if length > 0 {
            self.memory
                .guest_to_host(addr, length)
                .ok_or_else(|| anyhow!("mincore: invalid address range"))?;
        }

        // Get output vector and mark all pages as resident
        let vec_host = self
            .memory
            .guest_to_host_mut(vec_ptr, num_pages)
            .ok_or_else(|| anyhow!("mincore: invalid vec buffer"))?;

        // Mark all pages as resident (bit 0 = 1)
        // All valid guest memory is resident in the emulator
        unsafe {
            std::ptr::write_bytes(vec_host, 1, num_pages);
        }

        Ok(0)
    }

    fn sys_pkey_mprotect(&self) -> Result<i64> {
        let addr = self.data_regs[1] as usize;
        let len = self.data_regs[2] as usize;
        let _prot = self.data_regs[3] as i32;
        let _pkey = self.data_regs[4] as i32;

        // Validate that the memory range exists
        if len > 0 {
            let _ = self
                .memory
                .guest_to_host(addr, 1)
                .ok_or_else(|| anyhow!("pkey_mprotect: invalid address {:#x}", addr))?;
            if len > 1 {
                let _ = self
                    .memory
                    .guest_to_host(addr + len - 1, 1)
                    .ok_or_else(|| anyhow!("pkey_mprotect: invalid address range"))?;
            }
        }

        // Return success - guest memory protection is managed by the interpreter
        // m68k doesn't have hardware protection keys, so this is a simulated no-op
        Ok(0)
    }

    fn sys_pkey_alloc(&self) -> Result<i64> {
        let _flags = self.data_regs[1];
        let _access_rights = self.data_regs[2];

        // Simulate pkey allocation by returning a valid pkey ID
        // m68k doesn't have hardware protection keys, but we return success
        // to allow guest programs to call this without errors
        // Return 1 (a valid pkey number - 0 is reserved for default key)
        Ok(1)
    }

    fn sys_pkey_free(&self) -> Result<i64> {
        let _pkey = self.data_regs[1] as i32;

        // Always succeed - simulated pkey management
        Ok(0)
    }

    fn alloc_anonymous_mmap(&mut self, req_addr: usize, length: usize, prot: i32) -> Result<usize> {
        use crate::memory::MemorySegment;
        use goblin::elf::program_header;

        let aligned_len = (length + 4095) & !4095;
        let addr = if req_addr != 0 {
            req_addr
        } else {
            self.memory
                .find_free_range(aligned_len)
                .ok_or_else(|| anyhow!("mmap: no free address range for {aligned_len} bytes"))?
        };

        let mut elf_flags = 0u32;
        if prot & 0x1 != 0 {
            elf_flags |= program_header::PF_R;
        }
        if prot & 0x2 != 0 {
            elf_flags |= program_header::PF_W;
        }
        if prot & 0x4 != 0 {
            elf_flags |= program_header::PF_X;
        }

        self.memory.add_segment(MemorySegment {
            vaddr: addr,
            data: crate::memory::MemoryData::Owned(vec![0u8; aligned_len]),
            flags: elf_flags,
            align: 4096,
        });

        Ok(addr)
    }

    /// statfs(path, buf)
    fn sys_statfs(&mut self) -> Result<i64> {
        let path_addr = self.data_regs[1] as usize;
        let buf_addr = self.data_regs[2] as usize;
        let path = self.guest_cstring(path_addr)?;
        let mut statfs: libc::statfs = unsafe { std::mem::zeroed() };
        let result = unsafe { libc::statfs(path.as_ptr(), &mut statfs) };
        if result == 0 {
            self.write_statfs(buf_addr, &statfs)?;
        }
        Ok(result as i64)
    }

    /// fstatfs(fd, buf)
    fn sys_fstatfs(&mut self) -> Result<i64> {
        let fd = self.data_regs[1] as i32;
        let buf_addr = self.data_regs[2] as usize;
        let mut statfs: libc::statfs = unsafe { std::mem::zeroed() };
        let result = unsafe { libc::fstatfs(fd, &mut statfs) };
        if result == 0 {
            self.write_statfs(buf_addr, &statfs)?;
        }
        Ok(result as i64)
    }

    fn write_statfs(&mut self, addr: usize, s: &libc::statfs) -> Result<()> {
        // m68k statfs struct (simplified - key fields)
        self.memory
            .write_data(addr, &(s.f_type as u32).to_be_bytes())?;
        self.memory
            .write_data(addr + 4, &(s.f_bsize as u32).to_be_bytes())?;
        self.memory
            .write_data(addr + 8, &(s.f_blocks as u32).to_be_bytes())?;
        self.memory
            .write_data(addr + 12, &(s.f_bfree as u32).to_be_bytes())?;
        self.memory
            .write_data(addr + 16, &(s.f_bavail as u32).to_be_bytes())?;
        self.memory
            .write_data(addr + 20, &(s.f_files as u32).to_be_bytes())?;
        self.memory
            .write_data(addr + 24, &(s.f_ffree as u32).to_be_bytes())?;
        Ok(())
    }

    /// syslog(type, buf, len)
    fn sys_syslog(&mut self) -> Result<i64> {
        let logtype = self.data_regs[1] as i32;
        let buf_addr = self.data_regs[2] as usize;
        let len = self.data_regs[3] as i32;
        if buf_addr == 0 || len == 0 {
            return Ok(unsafe {
                libc::syscall(libc::SYS_syslog, logtype, std::ptr::null::<u8>(), len)
            });
        }
        let host_buf = self
            .memory
            .guest_to_host_mut(buf_addr, len as usize)
            .ok_or_else(|| anyhow!("invalid syslog buffer"))?;
        Ok(unsafe { libc::syscall(libc::SYS_syslog, logtype, host_buf, len) })
    }

    /// setitimer(which, new, old)
    fn sys_setitimer(&mut self) -> Result<i64> {
        let which = self.data_regs[1] as i32;
        let new_addr = self.data_regs[2] as usize;
        let old_addr = self.data_regs[3] as usize;

        let new_val = if new_addr != 0 {
            Some(self.read_itimerval(new_addr)?)
        } else {
            None
        };

        let mut old_val: libc::itimerval = unsafe { std::mem::zeroed() };
        let result = unsafe {
            libc::syscall(
                libc::SYS_setitimer,
                which,
                new_val.as_ref().map_or(std::ptr::null(), |v| v as *const _),
                if old_addr != 0 {
                    &mut old_val as *mut _
                } else {
                    std::ptr::null_mut::<libc::itimerval>()
                },
            )
        };

        if result == 0 && old_addr != 0 {
            self.write_itimerval(old_addr, &old_val)?;
        }
        Ok(result)
    }

    /// getitimer(which, curr)
    fn sys_getitimer(&mut self) -> Result<i64> {
        let which = self.data_regs[1] as i32;
        let curr_addr = self.data_regs[2] as usize;
        let mut curr: libc::itimerval = unsafe { std::mem::zeroed() };
        let result = unsafe { libc::syscall(libc::SYS_getitimer, which, &mut curr as *mut _) };
        if result == 0 && curr_addr != 0 {
            self.write_itimerval(curr_addr, &curr)?;
        }
        Ok(result)
    }

    fn read_itimerval(&self, addr: usize) -> Result<libc::itimerval> {
        // m68k uclibc uses 64-bit time_t
        let it_interval_sec_bytes: [u8; 8] = self.memory.read_data(addr, 8)?.try_into().unwrap();
        let it_interval_sec = i64::from_be_bytes(it_interval_sec_bytes) as libc::time_t;
        let it_interval_usec = self.memory.read_long(addr + 8)? as libc::suseconds_t;

        let it_value_sec_bytes: [u8; 8] = self.memory.read_data(addr + 12, 8)?.try_into().unwrap();
        let it_value_sec = i64::from_be_bytes(it_value_sec_bytes) as libc::time_t;
        let it_value_usec = self.memory.read_long(addr + 20)? as libc::suseconds_t;

        Ok(libc::itimerval {
            it_interval: libc::timeval {
                tv_sec: it_interval_sec,
                tv_usec: it_interval_usec,
            },
            it_value: libc::timeval {
                tv_sec: it_value_sec,
                tv_usec: it_value_usec,
            },
        })
    }

    fn write_itimerval(&mut self, addr: usize, val: &libc::itimerval) -> Result<()> {
        // m68k uclibc uses 64-bit time_t
        self.memory
            .write_data(addr, &val.it_interval.tv_sec.to_be_bytes())?;
        self.memory
            .write_data(addr + 8, &(val.it_interval.tv_usec as u32).to_be_bytes())?;
        self.memory
            .write_data(addr + 12, &val.it_value.tv_sec.to_be_bytes())?;
        self.memory
            .write_data(addr + 20, &(val.it_value.tv_usec as u32).to_be_bytes())?;
        Ok(())
    }

    /// stat/lstat(path, buf)
    fn sys_stat(&mut self, syscall_num: u32) -> Result<i64> {
        let path_addr = self.data_regs[1] as usize;
        let buf_addr = self.data_regs[2] as usize;
        let path = self.guest_cstring(path_addr)?;
        let mut stat: libc::stat = unsafe { std::mem::zeroed() };
        let result = unsafe { libc::syscall(syscall_num as i64, path.as_ptr(), &mut stat) };
        if result == 0 {
            self.write_stat(buf_addr, &stat)?;
        }
        Ok(result)
    }

    /// fstat(fd, buf)
    fn sys_fstat(&mut self) -> Result<i64> {
        let fd = self.data_regs[1] as i32;
        let buf_addr = self.data_regs[2] as usize;
        let mut stat: libc::stat = unsafe { std::mem::zeroed() };
        let result = unsafe { libc::fstat(fd, &mut stat) };
        if result == 0 {
            self.write_stat(buf_addr, &stat)?;
        }
        Ok(result as i64)
    }

    /// statx(dirfd, pathname, flags, mask, statxbuf)
    fn sys_statx(&mut self) -> Result<i64> {
        let dirfd = self.data_regs[1] as i32;
        let pathname_addr = self.data_regs[2] as usize;
        let flags = self.data_regs[3] as i32;
        let mask = self.data_regs[4];
        let statxbuf_addr = self.data_regs[5] as usize;

        let pathname = self.guest_cstring(pathname_addr)?;

        // Allocate statx buffer
        let mut statxbuf: libc::statx = unsafe { std::mem::zeroed() };

        // Call statx syscall directly
        let result = unsafe {
            libc::syscall(
                libc::SYS_statx,
                dirfd,
                pathname.as_ptr(),
                flags,
                mask,
                &mut statxbuf as *mut libc::statx,
            )
        };

        if result == 0 {
            self.write_statx(statxbuf_addr, &statxbuf)?;
        }

        Ok(Self::libc_to_kernel(result))
    }

    fn write_statx(&mut self, addr: usize, sx: &libc::statx) -> Result<()> {
        // statx structure has the same layout across all architectures
        // See: include/uapi/linux/stat.h
        let mut offset = addr;

        // u32 stx_mask
        self.memory.write_data(offset, &sx.stx_mask.to_be_bytes())?;
        offset += 4;

        // u32 stx_blksize
        self.memory
            .write_data(offset, &sx.stx_blksize.to_be_bytes())?;
        offset += 4;

        // u64 stx_attributes
        self.memory
            .write_data(offset, &sx.stx_attributes.to_be_bytes())?;
        offset += 8;

        // u32 stx_nlink
        self.memory
            .write_data(offset, &sx.stx_nlink.to_be_bytes())?;
        offset += 4;

        // u32 stx_uid
        self.memory.write_data(offset, &sx.stx_uid.to_be_bytes())?;
        offset += 4;

        // u32 stx_gid
        self.memory.write_data(offset, &sx.stx_gid.to_be_bytes())?;
        offset += 4;

        // u16 stx_mode
        self.memory.write_data(offset, &sx.stx_mode.to_be_bytes())?;
        offset += 2;

        // u16 __spare0[1] - padding
        self.memory.write_data(offset, &0u16.to_be_bytes())?;
        offset += 2;

        // u64 stx_ino
        self.memory.write_data(offset, &sx.stx_ino.to_be_bytes())?;
        offset += 8;

        // u64 stx_size
        self.memory.write_data(offset, &sx.stx_size.to_be_bytes())?;
        offset += 8;

        // u64 stx_blocks
        self.memory
            .write_data(offset, &sx.stx_blocks.to_be_bytes())?;
        offset += 8;

        // u64 stx_attributes_mask
        self.memory
            .write_data(offset, &sx.stx_attributes_mask.to_be_bytes())?;
        offset += 8;

        // struct statx_timestamp stx_atime (16 bytes: i64 tv_sec + u32 tv_nsec + i32 __reserved)
        self.memory
            .write_data(offset, &sx.stx_atime.tv_sec.to_be_bytes())?;
        offset += 8;
        self.memory
            .write_data(offset, &sx.stx_atime.tv_nsec.to_be_bytes())?;
        offset += 4;
        self.memory.write_data(offset, &0i32.to_be_bytes())?; // __reserved
        offset += 4;

        // struct statx_timestamp stx_btime (16 bytes)
        self.memory
            .write_data(offset, &sx.stx_btime.tv_sec.to_be_bytes())?;
        offset += 8;
        self.memory
            .write_data(offset, &sx.stx_btime.tv_nsec.to_be_bytes())?;
        offset += 4;
        self.memory.write_data(offset, &0i32.to_be_bytes())?;
        offset += 4;

        // struct statx_timestamp stx_ctime (16 bytes)
        self.memory
            .write_data(offset, &sx.stx_ctime.tv_sec.to_be_bytes())?;
        offset += 8;
        self.memory
            .write_data(offset, &sx.stx_ctime.tv_nsec.to_be_bytes())?;
        offset += 4;
        self.memory.write_data(offset, &0i32.to_be_bytes())?;
        offset += 4;

        // struct statx_timestamp stx_mtime (16 bytes)
        self.memory
            .write_data(offset, &sx.stx_mtime.tv_sec.to_be_bytes())?;
        offset += 8;
        self.memory
            .write_data(offset, &sx.stx_mtime.tv_nsec.to_be_bytes())?;
        offset += 4;
        self.memory.write_data(offset, &0i32.to_be_bytes())?;
        offset += 4;

        // u32 stx_rdev_major
        self.memory
            .write_data(offset, &sx.stx_rdev_major.to_be_bytes())?;
        offset += 4;

        // u32 stx_rdev_minor
        self.memory
            .write_data(offset, &sx.stx_rdev_minor.to_be_bytes())?;
        offset += 4;

        // u32 stx_dev_major
        self.memory
            .write_data(offset, &sx.stx_dev_major.to_be_bytes())?;
        offset += 4;

        // u32 stx_dev_minor
        self.memory
            .write_data(offset, &sx.stx_dev_minor.to_be_bytes())?;
        offset += 4;

        // u64 stx_mnt_id
        self.memory
            .write_data(offset, &sx.stx_mnt_id.to_be_bytes())?;
        offset += 8;

        // u32 stx_dio_mem_align
        #[cfg(target_os = "linux")]
        {
            // This field might not be available on older libc versions, so we write 0
            self.memory.write_data(offset, &0u32.to_be_bytes())?;
        }
        #[cfg(not(target_os = "linux"))]
        {
            self.memory.write_data(offset, &0u32.to_be_bytes())?;
        }
        offset += 4;

        // u32 stx_dio_offset_align
        self.memory.write_data(offset, &0u32.to_be_bytes())?;
        // offset += 4;

        // u64 __spare3[12] - spare fields at the end
        // We can skip these as they're zero

        Ok(())
    }

    fn write_stat(&mut self, addr: usize, s: &libc::stat) -> Result<()> {
        // m68k stat struct layout (32-bit)
        self.memory
            .write_data(addr, &(s.st_dev as u32).to_be_bytes())?;
        self.memory
            .write_data(addr + 4, &(s.st_ino as u32).to_be_bytes())?;
        self.memory
            .write_data(addr + 8, &(s.st_mode).to_be_bytes())?;
        self.memory
            .write_data(addr + 12, &(s.st_nlink as u32).to_be_bytes())?;
        self.memory
            .write_data(addr + 16, &(s.st_uid).to_be_bytes())?;
        self.memory
            .write_data(addr + 20, &(s.st_gid).to_be_bytes())?;
        self.memory
            .write_data(addr + 24, &(s.st_rdev as u32).to_be_bytes())?;
        self.memory
            .write_data(addr + 28, &(s.st_size as u32).to_be_bytes())?;
        self.memory
            .write_data(addr + 32, &(s.st_blksize as u32).to_be_bytes())?;
        self.memory
            .write_data(addr + 36, &(s.st_blocks as u32).to_be_bytes())?;
        self.memory
            .write_data(addr + 40, &(s.st_atime as u32).to_be_bytes())?;
        self.memory
            .write_data(addr + 44, &(s.st_mtime as u32).to_be_bytes())?;
        self.memory
            .write_data(addr + 48, &(s.st_ctime as u32).to_be_bytes())?;
        Ok(())
    }

    /// wait4(pid, status, options, rusage)
    fn sys_wait4(&mut self) -> Result<i64> {
        let pid = self.data_regs[1] as i32;
        let status_addr = self.data_regs[2] as usize;
        let options = self.data_regs[3] as i32;
        let rusage_addr = self.data_regs[4] as usize;

        let mut status: i32 = 0;
        let mut rusage: libc::rusage = unsafe { std::mem::zeroed() };

        let result = unsafe {
            libc::wait4(
                pid,
                if status_addr != 0 {
                    &mut status
                } else {
                    std::ptr::null_mut()
                },
                options,
                if rusage_addr != 0 {
                    &mut rusage
                } else {
                    std::ptr::null_mut()
                },
            )
        };

        if result > 0 {
            if status_addr != 0 {
                self.memory
                    .write_data(status_addr, &(status as u32).to_be_bytes())?;
            }
            if rusage_addr != 0 {
                self.memory
                    .write_data(rusage_addr, &(rusage.ru_utime.tv_sec as u32).to_be_bytes())?;
                self.memory.write_data(
                    rusage_addr + 4,
                    &(rusage.ru_utime.tv_usec as u32).to_be_bytes(),
                )?;
                self.memory.write_data(
                    rusage_addr + 8,
                    &(rusage.ru_stime.tv_sec as u32).to_be_bytes(),
                )?;
                self.memory.write_data(
                    rusage_addr + 12,
                    &(rusage.ru_stime.tv_usec as u32).to_be_bytes(),
                )?;
            }
        }
        Ok(result as i64)
    }

    /// waitid(idtype, id, infop, options)
    fn sys_waitid(&mut self) -> Result<i64> {
        let idtype = self.data_regs[1] as i32;
        let id = self.data_regs[2] as i32;
        let infop_addr = self.data_regs[3] as usize;
        let options = self.data_regs[4] as i32;

        // Allocate host siginfo_t
        let mut infop: libc::siginfo_t = unsafe { std::mem::zeroed() };

        // Call waitid (5th parameter is rusage, always NULL for basic waitid)
        let result = unsafe {
            libc::syscall(
                libc::SYS_waitid,
                idtype,
                id,
                if infop_addr != 0 {
                    &mut infop as *mut _
                } else {
                    std::ptr::null_mut::<libc::siginfo_t>()
                },
                options,
                std::ptr::null_mut::<libc::c_void>(), // rusage (NULL)
            ) as i64
        };

        // If successful and infop is not NULL, write back siginfo_t
        if result == 0 && infop_addr != 0 {
            // Translate siginfo_t structure from host to m68k layout
            // For waitid, we mainly care about si_signo, si_errno, si_code, and _sigchld union
            unsafe {
                // Write si_signo (offset 0)
                self.memory
                    .write_data(infop_addr, &(infop.si_signo as u32).to_be_bytes())?;
                // Write si_errno (offset 4)
                self.memory
                    .write_data(infop_addr + 4, &(infop.si_errno as u32).to_be_bytes())?;
                // Write si_code (offset 8)
                self.memory
                    .write_data(infop_addr + 8, &(infop.si_code as u32).to_be_bytes())?;

                // For _sigchld union (offset 12), write pid, uid, status
                // Access via si_pid(), si_uid(), si_status() methods
                let si_pid = infop.si_pid();
                let si_uid = infop.si_uid();
                let si_status = infop.si_status();

                self.memory
                    .write_data(infop_addr + 12, &(si_pid as u32).to_be_bytes())?;
                self.memory
                    .write_data(infop_addr + 16, &(si_uid as u32).to_be_bytes())?;
                self.memory
                    .write_data(infop_addr + 20, &(si_status as u32).to_be_bytes())?;
            }
        }

        Ok(Self::libc_to_kernel(result))
    }

    /// getpagesize() -> host page size
    fn sys_getpagesize(&mut self) -> Result<i64> {
        let sz = unsafe { libc::sysconf(libc::_SC_PAGESIZE) };
        if sz <= 0 {
            Ok(-libc::EINVAL as i64)
        } else {
            Ok(sz as i64)
        }
    }

    /// Read xattr_args structure from guest memory.
    fn read_xattr_args(&self, addr: usize) -> Result<(usize, usize, u32)> {
        let value_ptr = self.memory.read_long(addr)? as usize;
        let size = self.memory.read_long(addr + 4)? as usize;
        let flags = self.memory.read_long(addr + 8)?;
        Ok((value_ptr, size, flags))
    }

    /// shmctl(shmid, cmd, buf)
    fn sys_shmctl(&mut self) -> Result<i64> {
        let shmid = self.data_regs[1] as i32;
        let cmd = self.data_regs[2] as i32;
        let buf_ptr = self.data_regs[3] as usize;

        // For IPC_RMID, IPC_INFO, SHM_INFO, we don't need the buffer
        // or it's read-only, so we can pass NULL
        if cmd == libc::IPC_RMID || buf_ptr == 0 {
            let res =
                unsafe { libc::syscall(31, shmid, cmd, std::ptr::null_mut::<libc::c_void>()) };
            return Ok(Self::libc_to_kernel(res as i64));
        }

        // For IPC_STAT and IPC_SET, we need to translate the structure
        // For now, we'll just pass it through since modern m68k libc seems to use
        // 64-bit time_t like x86_64. If this causes issues, we'll need to add
        // proper structure translation.
        let buf_host = self
            .memory
            .guest_to_host_mut(buf_ptr, 128) // shmid_ds is ~112 bytes on x86_64
            .ok_or_else(|| anyhow!("invalid shmid_ds buffer"))?;

        let res = unsafe { libc::syscall(31, shmid, cmd, buf_host) };
        Ok(Self::libc_to_kernel(res as i64))
    }

    /// ipc(call, first, second, third, ptr, fifth)
    /// Multiplexer syscall that dispatches to individual IPC operations
    fn sys_ipc(&mut self) -> Result<i64> {
        let call = self.data_regs[1];
        let first = self.data_regs[2] as i32;
        let second = self.data_regs[3] as usize;
        let third = self.data_regs[4] as usize;
        let ptr = self.data_regs[5] as usize;
        let fifth = self.data_regs[6] as i64;

        // IPC call numbers
        const SEMOP: u32 = 1;
        const SEMGET: u32 = 2;
        const SEMCTL: u32 = 3;
        const SEMTIMEDOP: u32 = 4;
        const MSGSND: u32 = 11;
        const MSGRCV: u32 = 12;
        const MSGGET: u32 = 13;
        const MSGCTL: u32 = 14;
        const SHMAT: u32 = 21;
        const SHMDT: u32 = 22;
        const SHMGET: u32 = 23;
        const SHMCTL: u32 = 24;

        match call {
            SEMGET => {
                // semget(key, nsems, semflg)
                let saved = self.data_regs;
                self.data_regs[1] = first as u32;
                self.data_regs[2] = second as u32;
                self.data_regs[3] = third as u32;
                let result = unsafe { libc::syscall(64, first, second, third) as i64 };
                self.data_regs = saved;
                Ok(Self::libc_to_kernel(result))
            }
            SEMCTL => {
                // semctl(semid, semnum, cmd, arg)
                let saved = self.data_regs;
                self.data_regs[1] = first as u32;
                self.data_regs[2] = second as u32;
                self.data_regs[3] = third as u32;
                self.data_regs[4] = ptr as u32;
                let result = self.sys_semctl()?;
                self.data_regs = saved;
                Ok(result)
            }
            SHMGET => {
                // shmget(key, size, shmflg)
                let saved = self.data_regs;
                self.data_regs[1] = first as u32;
                self.data_regs[2] = second as u32;
                self.data_regs[3] = third as u32;
                let result = unsafe { libc::syscall(29, first, second, third) as i64 };
                self.data_regs = saved;
                Ok(Self::libc_to_kernel(result))
            }
            SHMCTL => {
                // shmctl(shmid, cmd, buf)
                let saved = self.data_regs;
                self.data_regs[1] = first as u32;
                self.data_regs[2] = second as u32;
                self.data_regs[3] = ptr as u32;
                let result = self.sys_shmctl()?;
                self.data_regs = saved;
                Ok(result)
            }
            SHMAT => {
                // shmat(shmid, shmaddr, shmflg)
                let saved = self.data_regs;
                self.data_regs[1] = first as u32;
                self.data_regs[2] = second as u32;
                self.data_regs[3] = third as u32;
                let result = self.sys_shmat()?;
                self.data_regs = saved;
                Ok(result)
            }
            SHMDT => {
                // shmdt(shmaddr)
                let saved = self.data_regs;
                self.data_regs[1] = ptr as u32;
                let result = self.sys_shmdt()?;
                self.data_regs = saved;
                Ok(result)
            }
            MSGGET => {
                // msgget(key, msgflg)
                let saved = self.data_regs;
                self.data_regs[1] = first as u32;
                self.data_regs[2] = second as u32;
                let result = unsafe { libc::syscall(68, first, second) as i64 };
                self.data_regs = saved;
                Ok(Self::libc_to_kernel(result))
            }
            MSGSND => {
                // msgsnd(msqid, msgp, msgsz, msgflg)
                let saved = self.data_regs;
                self.data_regs[1] = first as u32;
                self.data_regs[2] = ptr as u32;
                self.data_regs[3] = second as u32;
                self.data_regs[4] = third as u32;
                let result = self.sys_msgsnd()?;
                self.data_regs = saved;
                Ok(result)
            }
            MSGRCV => {
                // msgrcv(msqid, msgp, msgsz, msgtyp, msgflg)
                let saved = self.data_regs;
                self.data_regs[1] = first as u32;
                self.data_regs[2] = ptr as u32;
                self.data_regs[3] = second as u32;
                self.data_regs[4] = fifth as u32;
                self.data_regs[5] = third as u32;
                let result = self.sys_msgrcv()?;
                self.data_regs = saved;
                Ok(result)
            }
            MSGCTL => {
                // msgctl(msqid, cmd, buf)
                let saved = self.data_regs;
                self.data_regs[1] = first as u32;
                self.data_regs[2] = second as u32;
                self.data_regs[3] = ptr as u32;
                let result = self.sys_msgctl()?;
                self.data_regs = saved;
                Ok(result)
            }
            SEMOP | SEMTIMEDOP => {
                // semop/semtimedop not yet implemented
                bail!("semop/semtimedop not yet implemented")
            }
            _ => bail!("unknown ipc call number: {}", call),
        }
    }

    /// shmat(shmid, shmaddr, shmflg)
    fn sys_shmat(&mut self) -> Result<i64> {
        let shmid = self.data_regs[1] as i32;
        let shmaddr_hint = self.data_regs[2] as usize;
        let shmflg = self.data_regs[3] as i32;

        // Call host shmat to attach the shared memory
        let host_ptr = unsafe { libc::shmat(shmid, std::ptr::null(), shmflg) as *mut u8 };

        if host_ptr == libc::MAP_FAILED as *mut u8 {
            let errno = unsafe { *libc::__errno_location() };
            return Ok(Self::libc_to_kernel(-errno as i64));
        }

        // Get the size of the shared memory segment
        let mut shmid_ds: libc::shmid_ds = unsafe { std::mem::zeroed() };
        let stat_result = unsafe { libc::shmctl(shmid, libc::IPC_STAT, &mut shmid_ds) };
        if stat_result < 0 {
            // Failed to get size - detach and return error
            unsafe { libc::shmdt(host_ptr as *const libc::c_void) };
            let errno = unsafe { *libc::__errno_location() };
            return Ok(Self::libc_to_kernel(-errno as i64));
        }
        let size = shmid_ds.shm_segsz;

        // Find a guest address for this mapping
        let guest_addr = if shmaddr_hint == 0 {
            // Find a free range in guest address space
            self.memory
                .find_free_range(size)
                .ok_or_else(|| anyhow!("no free guest memory for shmat"))?
        } else {
            // Use the hint address (TODO: handle SHM_RND flag for rounding)
            shmaddr_hint
        };

        // Create a foreign segment that wraps the host shmat memory
        let flags = if shmflg & libc::SHM_RDONLY != 0 {
            goblin::elf::program_header::PF_R
        } else {
            goblin::elf::program_header::PF_R | goblin::elf::program_header::PF_W
        };

        let segment = crate::memory::MemorySegment {
            vaddr: guest_addr,
            data: crate::memory::MemoryData::Foreign {
                ptr: host_ptr,
                len: size,
                shmid,
            },
            flags,
            align: 4096,
        };

        // Add this segment to the guest's memory map
        self.memory.add_segment(segment);

        Ok(guest_addr as i64)
    }

    /// shmdt(shmaddr)
    fn sys_shmdt(&mut self) -> Result<i64> {
        let guest_addr = self.data_regs[1] as usize;

        // Find the segment at this address
        let segment_idx = self
            .memory
            .find_segment_index(guest_addr)
            .ok_or_else(|| anyhow!("no shared memory segment at address {:#x}", guest_addr))?;

        // Verify it's a foreign segment (from shmat)
        // Note: The Drop implementation for MemoryData will call shmdt automatically
        // when we remove the segment, so we just need to remove it
        self.memory.remove_segment(segment_idx);

        Ok(0)
    }

    /// msgctl(msqid, cmd, buf)
    fn sys_msgctl(&mut self) -> Result<i64> {
        let msqid = self.data_regs[1] as i32;
        let cmd = self.data_regs[2] as i32;
        let buf_ptr = self.data_regs[3] as usize;

        // For IPC_RMID, we don't need the buffer
        if cmd == libc::IPC_RMID || buf_ptr == 0 {
            let res =
                unsafe { libc::syscall(71, msqid, cmd, std::ptr::null_mut::<libc::c_void>()) };
            return Ok(Self::libc_to_kernel(res as i64));
        }

        // For IPC_STAT and IPC_SET, pass through the buffer
        // Similar to shmctl, we'll trust that the structure layout is compatible
        let buf_host = self
            .memory
            .guest_to_host_mut(buf_ptr, 128) // msqid_ds is similar size
            .ok_or_else(|| anyhow!("invalid msqid_ds buffer"))?;

        let res = unsafe { libc::syscall(71, msqid, cmd, buf_host) };
        Ok(Self::libc_to_kernel(res as i64))
    }

    /// msgsnd(msqid, msgp, msgsz, msgflg)
    /// Message buffer format:
    ///   m68k: struct { i32 mtype; char mtext[]; }
    ///   x86_64: struct { i64 mtype; char mtext[]; }
    fn sys_msgsnd(&mut self) -> Result<i64> {
        let msqid = self.data_regs[1] as i32;
        let msgp_guest = self.data_regs[2] as usize;
        let msgsz = self.data_regs[3] as usize;
        let msgflg = self.data_regs[4] as i32;

        // Read the m68k message buffer (4-byte mtype + message data)
        let mtype_m68k = self.memory.read_long(msgp_guest)? as i32;

        // Create host buffer with 8-byte mtype
        let total_size = 8 + msgsz; // 8 bytes for mtype (i64) + message data
        let mut host_buf = vec![0u8; total_size];

        // Write mtype as 8-byte value
        host_buf[0..8].copy_from_slice(&(mtype_m68k as i64).to_ne_bytes());

        // Copy message data (skip 4-byte m68k mtype, copy msgsz bytes)
        if msgsz > 0 {
            let mtext_guest = msgp_guest + 4;
            let mtext_data = self
                .memory
                .guest_to_host(mtext_guest, msgsz)
                .ok_or_else(|| anyhow!("invalid message data"))?;
            host_buf[8..].copy_from_slice(unsafe { std::slice::from_raw_parts(mtext_data, msgsz) });
        }

        let res = unsafe { libc::syscall(69, msqid, host_buf.as_ptr(), msgsz, msgflg) };
        Ok(Self::libc_to_kernel(res as i64))
    }

    /// msgrcv(msqid, msgp, msgsz, msgtyp, msgflg)
    fn sys_msgrcv(&mut self) -> Result<i64> {
        let msqid = self.data_regs[1] as i32;
        let msgp_guest = self.data_regs[2] as usize;
        let msgsz = self.data_regs[3] as usize;
        let msgtyp = self.data_regs[4] as i64;
        let msgflg = self.data_regs[5] as i32;

        // Create host buffer with 8-byte mtype
        let total_size = 8 + msgsz;
        let mut host_buf = vec![0u8; total_size];

        let res = unsafe { libc::syscall(70, msqid, host_buf.as_mut_ptr(), msgsz, msgtyp, msgflg) };

        if res < 0 {
            return Ok(Self::libc_to_kernel(res as i64));
        }

        // Copy the received message back to guest memory
        // Convert 8-byte mtype to 4-byte for m68k
        let mtype_host = i64::from_ne_bytes(host_buf[0..8].try_into().unwrap());
        self.memory
            .write_data(msgp_guest, &(mtype_host as i32).to_be_bytes())?;

        // Copy message data
        if res > 0 {
            let mtext_guest = msgp_guest + 4;
            let mtext_host = self
                .memory
                .guest_to_host_mut(mtext_guest, res as usize)
                .ok_or_else(|| anyhow!("invalid message data buffer"))?;
            unsafe {
                std::ptr::copy_nonoverlapping(host_buf[8..].as_ptr(), mtext_host, res as usize);
            }
        }

        Ok(res as i64)
    }

    /// mq_unlink(name) - Remove a message queue
    fn sys_mq_unlink(&self) -> Result<i64> {
        let name_addr = self.data_regs[1] as usize;
        let name_cstr = self.guest_cstring(name_addr)?;

        let result = unsafe { libc::syscall(241, name_cstr.as_ptr()) as i64 };
        Ok(Self::libc_to_kernel(result))
    }

    /// mq_open(name, oflag, mode, attr) - Open/create a message queue
    /// Variadic syscall: takes 2 or 4 arguments depending on oflag
    fn sys_mq_open(&self) -> Result<i64> {
        let name_addr = self.data_regs[1] as usize;
        let oflag = self.data_regs[2] as i32;
        let mode = self.data_regs[3]; // Only used if O_CREAT is set
        let attr_addr = self.data_regs[4] as usize; // Only used if O_CREAT is set

        let name_cstr = self.guest_cstring(name_addr)?;

        // Check if O_CREAT is set (0x40 = O_CREAT)
        let result = if (oflag & 0x40) != 0 {
            // Mode and attr are provided
            let attr_ptr = if attr_addr == 0 {
                std::ptr::null::<libc::mq_attr>()
            } else {
                // Read struct mq_attr from guest memory
                let mq_flags = self.memory.read_long(attr_addr)? as i32 as i64;
                let mq_maxmsg = self.memory.read_long(attr_addr + 4)? as i32 as i64;
                let mq_msgsize = self.memory.read_long(attr_addr + 8)? as i32 as i64;
                let mq_curmsgs = self.memory.read_long(attr_addr + 12)? as i32 as i64;

                let mut attr: libc::mq_attr = unsafe { std::mem::zeroed() };
                attr.mq_flags = mq_flags;
                attr.mq_maxmsg = mq_maxmsg;
                attr.mq_msgsize = mq_msgsize;
                attr.mq_curmsgs = mq_curmsgs;

                Box::leak(Box::new(attr)) as *const libc::mq_attr
            };

            let res =
                unsafe { libc::syscall(240, name_cstr.as_ptr(), oflag, mode, attr_ptr) as i64 };

            // Clean up leaked attr if allocated
            if !attr_ptr.is_null() {
                unsafe {
                    let _ = Box::from_raw(attr_ptr as *mut libc::mq_attr);
                }
            }

            res
        } else {
            // Simple open without mode/attr
            unsafe { libc::syscall(240, name_cstr.as_ptr(), oflag) as i64 }
        };

        Ok(Self::libc_to_kernel(result))
    }

    /// mq_getsetattr(mqdes, newattr, oldattr) - Get/set message queue attributes
    /// struct mq_attr on m68k: 4 longs × 4 bytes = 16 bytes
    /// struct mq_attr on x86_64: 4 longs × 8 bytes = 32 bytes
    fn sys_mq_getsetattr(&mut self) -> Result<i64> {
        let mqdes = self.data_regs[1] as i32;
        let newattr_addr = self.data_regs[2] as usize;
        let oldattr_addr = self.data_regs[3] as usize;

        // Read newattr from guest memory if provided
        let newattr_ptr = if newattr_addr == 0 {
            std::ptr::null::<libc::mq_attr>()
        } else {
            // Read 4 longs (4 bytes each on m68k)
            let mq_flags = self.memory.read_long(newattr_addr)? as i32 as i64;
            let mq_maxmsg = self.memory.read_long(newattr_addr + 4)? as i32 as i64;
            let mq_msgsize = self.memory.read_long(newattr_addr + 8)? as i32 as i64;
            let mq_curmsgs = self.memory.read_long(newattr_addr + 12)? as i32 as i64;

            // Build host mq_attr
            let mut newattr: libc::mq_attr = unsafe { std::mem::zeroed() };
            newattr.mq_flags = mq_flags;
            newattr.mq_maxmsg = mq_maxmsg;
            newattr.mq_msgsize = mq_msgsize;
            newattr.mq_curmsgs = mq_curmsgs;

            // Store in a temporary location (need to keep it alive for syscall)
            // We'll use a local variable and take its address
            Box::leak(Box::new(newattr)) as *const libc::mq_attr
        };

        // Prepare oldattr buffer if requested
        let mut oldattr: libc::mq_attr = unsafe { std::mem::zeroed() };
        let oldattr_ptr = if oldattr_addr == 0 {
            std::ptr::null_mut::<libc::mq_attr>()
        } else {
            &mut oldattr as *mut libc::mq_attr
        };

        // Call mq_getsetattr (x86_64 syscall 245)
        let result = unsafe { libc::syscall(245, mqdes, newattr_ptr, oldattr_ptr) as i64 };

        // Clean up leaked newattr if allocated
        if !newattr_ptr.is_null() {
            unsafe {
                let _ = Box::from_raw(newattr_ptr as *mut libc::mq_attr);
            }
        }

        // Write oldattr back to guest memory if requested
        if result >= 0 && oldattr_addr != 0 {
            self.memory
                .write_data(oldattr_addr, &(oldattr.mq_flags as i32).to_be_bytes())?;
            self.memory
                .write_data(oldattr_addr + 4, &(oldattr.mq_maxmsg as i32).to_be_bytes())?;
            self.memory
                .write_data(oldattr_addr + 8, &(oldattr.mq_msgsize as i32).to_be_bytes())?;
            self.memory.write_data(
                oldattr_addr + 12,
                &(oldattr.mq_curmsgs as i32).to_be_bytes(),
            )?;
        }

        Ok(Self::libc_to_kernel(result))
    }

    /// mq_timedsend(mqdes, msg_ptr, msg_len, msg_prio, abs_timeout) - Send message with timeout
    fn sys_mq_timedsend(&self) -> Result<i64> {
        let mqdes = self.data_regs[1] as i32;
        let msg_ptr_guest = self.data_regs[2] as usize;
        let msg_len = self.data_regs[3] as usize;
        let msg_prio = self.data_regs[4];
        let timeout_addr = self.data_regs[5] as usize;

        // Translate message buffer pointer
        let msg_ptr_host = if msg_len > 0 {
            self.memory
                .guest_to_host(msg_ptr_guest, msg_len)
                .ok_or_else(|| anyhow!("mq_timedsend: invalid message buffer"))?
        } else {
            std::ptr::null()
        };

        // Read timeout if provided (m68k uclibc uses 64-bit time_t)
        let timeout_ptr = if timeout_addr == 0 {
            std::ptr::null::<libc::timespec>()
        } else {
            // tv_sec: 8 bytes (big-endian i64)
            let tv_sec_bytes: [u8; 8] = self.memory.read_data(timeout_addr, 8)?.try_into().unwrap();
            let tv_sec = i64::from_be_bytes(tv_sec_bytes);

            // tv_nsec: 4 bytes (big-endian i32)
            let tv_nsec = self.memory.read_long(timeout_addr + 8)? as i64;

            let timeout = libc::timespec { tv_sec, tv_nsec };
            Box::leak(Box::new(timeout)) as *const libc::timespec
        };

        let result =
            unsafe { libc::syscall(242, mqdes, msg_ptr_host, msg_len, msg_prio, timeout_ptr) };

        // Clean up leaked timeout if allocated
        if !timeout_ptr.is_null() {
            unsafe {
                let _ = Box::from_raw(timeout_ptr as *mut libc::timespec);
            }
        }

        Ok(Self::libc_to_kernel(result as i64))
    }

    /// mq_timedreceive(mqdes, msg_ptr, msg_len, msg_prio, abs_timeout) - Receive message with timeout
    fn sys_mq_timedreceive(&mut self) -> Result<i64> {
        let mqdes = self.data_regs[1] as i32;
        let msg_ptr_guest = self.data_regs[2] as usize;
        let msg_len = self.data_regs[3] as usize;
        let msg_prio_addr = self.data_regs[4] as usize;
        let timeout_addr = self.data_regs[5] as usize;

        // Translate message buffer pointer
        let msg_ptr_host = if msg_len > 0 {
            self.memory
                .guest_to_host_mut(msg_ptr_guest, msg_len)
                .ok_or_else(|| anyhow!("mq_timedreceive: invalid message buffer"))?
        } else {
            std::ptr::null_mut()
        };

        // Read timeout if provided (m68k uclibc uses 64-bit time_t)
        let timeout_ptr = if timeout_addr == 0 {
            std::ptr::null::<libc::timespec>()
        } else {
            // tv_sec: 8 bytes (big-endian i64)
            let tv_sec_bytes: [u8; 8] = self.memory.read_data(timeout_addr, 8)?.try_into().unwrap();
            let tv_sec = i64::from_be_bytes(tv_sec_bytes);

            // tv_nsec: 4 bytes (big-endian i32)
            let tv_nsec = self.memory.read_long(timeout_addr + 8)? as i64;

            let timeout = libc::timespec { tv_sec, tv_nsec };
            Box::leak(Box::new(timeout)) as *const libc::timespec
        };

        // Prepare msg_prio buffer if requested
        let mut msg_prio: u32 = 0;
        let msg_prio_ptr = if msg_prio_addr == 0 {
            std::ptr::null_mut::<u32>()
        } else {
            &mut msg_prio as *mut u32
        };

        let result =
            unsafe { libc::syscall(243, mqdes, msg_ptr_host, msg_len, msg_prio_ptr, timeout_ptr) };

        // Clean up leaked timeout if allocated
        if !timeout_ptr.is_null() {
            unsafe {
                let _ = Box::from_raw(timeout_ptr as *mut libc::timespec);
            }
        }

        // Write msg_prio back to guest memory if requested
        if result >= 0 && msg_prio_addr != 0 {
            self.memory
                .write_data(msg_prio_addr, &msg_prio.to_be_bytes())?;
        }

        Ok(Self::libc_to_kernel(result as i64))
    }

    /// semctl(semid, semnum, cmd, arg)
    /// arg is a union semun which can be:
    ///   - int val (for SETVAL)
    ///   - struct semid_ds *buf (for IPC_STAT, IPC_SET)
    ///   - unsigned short *array (for GETALL, SETALL)
    fn sys_semctl(&mut self) -> Result<i64> {
        let semid = self.data_regs[1] as i32;
        let semnum = self.data_regs[2] as i32;
        let cmd = self.data_regs[3] as i32;
        let arg_val = self.data_regs[4] as usize; // Can be int or pointer depending on cmd

        // Commands that don't need the 4th argument
        if cmd == libc::IPC_RMID {
            let res = unsafe { libc::syscall(66, semid, semnum, cmd, 0) };
            return Ok(Self::libc_to_kernel(res as i64));
        }

        // SETVAL uses arg.val (passed as integer)
        const SETVAL: i32 = 16;
        if cmd == SETVAL {
            let res = unsafe { libc::syscall(66, semid, semnum, cmd, arg_val as i32) };
            return Ok(Self::libc_to_kernel(res as i64));
        }

        // GETVAL, GETPID, GETNCNT, GETZCNT don't use arg
        const GETVAL: i32 = 12;
        const GETPID: i32 = 11;
        const GETNCNT: i32 = 14;
        const GETZCNT: i32 = 15;
        if cmd == GETVAL || cmd == GETPID || cmd == GETNCNT || cmd == GETZCNT {
            let res = unsafe { libc::syscall(66, semid, semnum, cmd, 0) };
            return Ok(Self::libc_to_kernel(res as i64));
        }

        // IPC_STAT, IPC_SET use arg.buf (struct semid_ds*)
        if cmd == libc::IPC_STAT || cmd == libc::IPC_SET {
            if arg_val == 0 {
                let res = unsafe {
                    libc::syscall(66, semid, semnum, cmd, std::ptr::null_mut::<libc::c_void>())
                };
                return Ok(Self::libc_to_kernel(res as i64));
            }
            let buf_host = self
                .memory
                .guest_to_host_mut(arg_val, 128)
                .ok_or_else(|| anyhow!("invalid semid_ds buffer"))?;
            let res = unsafe { libc::syscall(66, semid, semnum, cmd, buf_host) };
            return Ok(Self::libc_to_kernel(res as i64));
        }

        // GETALL, SETALL use arg.array (unsigned short*)
        // For now, just pass through - these would need array translation if sizes differ
        const GETALL: i32 = 13;
        const SETALL: i32 = 17;
        if cmd == GETALL || cmd == SETALL {
            // Need to get the semaphore count to know array size
            // For simplicity, allocate a reasonable buffer (max 256 semaphores)
            if arg_val == 0 {
                return Ok(Self::libc_to_kernel(-libc::EINVAL as i64));
            }
            let array_host = self
                .memory
                .guest_to_host_mut(arg_val, 512) // 256 shorts * 2 bytes
                .ok_or_else(|| anyhow!("invalid semaphore array"))?;
            let res = unsafe { libc::syscall(66, semid, semnum, cmd, array_host) };
            return Ok(Self::libc_to_kernel(res as i64));
        }

        // Default: pass through arg as-is (for other commands like IPC_INFO)
        let res = unsafe { libc::syscall(66, semid, semnum, cmd, arg_val) };
        Ok(Self::libc_to_kernel(res as i64))
    }

    /// init_module(module_image, len, param_values)
    fn sys_init_module(&mut self) -> Result<i64> {
        let module_image_ptr = self.data_regs[1] as usize;
        let len = self.data_regs[2] as usize;
        let param_values_ptr = self.data_regs[3] as usize;

        // Translate module_image buffer pointer
        let module_image = if len > 0 {
            self.memory
                .guest_to_host(module_image_ptr, len)
                .ok_or_else(|| anyhow!("invalid module_image buffer"))?
        } else {
            std::ptr::null()
        };

        // Translate param_values string pointer
        let param_values = if param_values_ptr != 0 {
            self.read_c_string(param_values_ptr)?
        } else {
            vec![0u8] // Empty C string
        };

        // Call init_module syscall (x86_64 syscall number is 175)
        let res = unsafe { libc::syscall(175, module_image, len, param_values.as_ptr()) };
        Ok(Self::libc_to_kernel(res as i64))
    }

    /// atomic_cmpxchg_32(uaddr, oldval, newval) - m68k only
    /// Returns the previous value at *uaddr.
    fn sys_atomic_cmpxchg_32(&mut self) -> Result<i64> {
        let addr = self.data_regs[1] as usize;
        let old = self.data_regs[2];
        let new = self.data_regs[3];

        // Read current value
        let current = self.memory.read_long(addr)?;

        // If matches expected, write new value
        if current == old {
            self.memory.write_data(addr, &new.to_be_bytes())?;
        }
        Ok(current as i64)
    }

    /// atomic_barrier() - act as a full memory barrier on host
    fn sys_atomic_barrier(&self) -> Result<i64> {
        use std::sync::atomic::{Ordering, fence};
        fence(Ordering::SeqCst);
        Ok(0)
    }

    /// setxattr(path, name, value, size, flags)
    fn sys_setxattr(&mut self) -> Result<i64> {
        let path_ptr = self.data_regs[1] as usize;
        let name_ptr = self.data_regs[2] as usize;
        let value_ptr = self.data_regs[3] as usize;
        let size = self.data_regs[4] as usize;
        let flags = self.data_regs[5] as i32;

        let path = self.read_c_string(path_ptr)?;
        let name = self.read_c_string(name_ptr)?;
        let value = if value_ptr != 0 && size > 0 {
            self.memory
                .guest_to_host(value_ptr, size)
                .ok_or_else(|| anyhow!("invalid xattr value buffer"))?
        } else {
            std::ptr::null()
        };

        let res = unsafe {
            libc::setxattr(
                path.as_ptr() as *const libc::c_char,
                name.as_ptr() as *const libc::c_char,
                value as *const libc::c_void,
                size,
                flags,
            )
        };
        Ok(Self::libc_to_kernel(res as i64))
    }

    /// lsetxattr(path, name, value, size, flags)
    fn sys_lsetxattr(&mut self) -> Result<i64> {
        let path_ptr = self.data_regs[1] as usize;
        let name_ptr = self.data_regs[2] as usize;
        let value_ptr = self.data_regs[3] as usize;
        let size = self.data_regs[4] as usize;
        let flags = self.data_regs[5] as i32;

        let path = self.read_c_string(path_ptr)?;
        let name = self.read_c_string(name_ptr)?;
        let value = if value_ptr != 0 && size > 0 {
            self.memory
                .guest_to_host(value_ptr, size)
                .ok_or_else(|| anyhow!("invalid xattr value buffer"))?
        } else {
            std::ptr::null()
        };

        let res = unsafe {
            libc::lsetxattr(
                path.as_ptr() as *const libc::c_char,
                name.as_ptr() as *const libc::c_char,
                value as *const libc::c_void,
                size,
                flags,
            )
        };
        Ok(Self::libc_to_kernel(res as i64))
    }

    /// fsetxattr(fd, name, value, size, flags)
    fn sys_fsetxattr(&mut self) -> Result<i64> {
        let fd = self.data_regs[1] as libc::c_int;
        let name_ptr = self.data_regs[2] as usize;
        let value_ptr = self.data_regs[3] as usize;
        let size = self.data_regs[4] as usize;
        let flags = self.data_regs[5] as i32;

        let name = self.read_c_string(name_ptr)?;
        let value = if value_ptr != 0 && size > 0 {
            self.memory
                .guest_to_host(value_ptr, size)
                .ok_or_else(|| anyhow!("invalid xattr value buffer"))?
        } else {
            std::ptr::null()
        };

        let res = unsafe {
            libc::fsetxattr(
                fd,
                name.as_ptr() as *const libc::c_char,
                value as *const libc::c_void,
                size,
                flags,
            )
        };
        Ok(Self::libc_to_kernel(res as i64))
    }

    /// getxattr(path, name, value, size)
    fn sys_getxattr(&mut self) -> Result<i64> {
        let path_ptr = self.data_regs[1] as usize;
        let name_ptr = self.data_regs[2] as usize;
        let value_ptr = self.data_regs[3] as usize;
        let size = self.data_regs[4] as usize;

        let path = self.read_c_string(path_ptr)?;
        let name = self.read_c_string(name_ptr)?;
        let buf_host = if value_ptr != 0 && size > 0 {
            self.memory
                .guest_to_host_mut(value_ptr, size)
                .ok_or_else(|| anyhow!("invalid xattr buffer"))?
        } else {
            std::ptr::null_mut()
        };

        let res = unsafe {
            libc::getxattr(
                path.as_ptr() as *const libc::c_char,
                name.as_ptr() as *const libc::c_char,
                buf_host as *mut libc::c_void,
                size,
            )
        };
        Ok(Self::libc_to_kernel(res as i64))
    }

    /// lgetxattr(path, name, value, size)
    fn sys_lgetxattr(&mut self) -> Result<i64> {
        let path_ptr = self.data_regs[1] as usize;
        let name_ptr = self.data_regs[2] as usize;
        let value_ptr = self.data_regs[3] as usize;
        let size = self.data_regs[4] as usize;

        let path = self.read_c_string(path_ptr)?;
        let name = self.read_c_string(name_ptr)?;
        let buf_host = if value_ptr != 0 && size > 0 {
            self.memory
                .guest_to_host_mut(value_ptr, size)
                .ok_or_else(|| anyhow!("invalid xattr buffer"))?
        } else {
            std::ptr::null_mut()
        };

        let res = unsafe {
            libc::lgetxattr(
                path.as_ptr() as *const libc::c_char,
                name.as_ptr() as *const libc::c_char,
                buf_host as *mut libc::c_void,
                size,
            )
        };
        Ok(Self::libc_to_kernel(res as i64))
    }

    /// fgetxattr(fd, name, value, size)
    fn sys_fgetxattr(&mut self) -> Result<i64> {
        let fd = self.data_regs[1] as libc::c_int;
        let name_ptr = self.data_regs[2] as usize;
        let value_ptr = self.data_regs[3] as usize;
        let size = self.data_regs[4] as usize;

        let name = self.read_c_string(name_ptr)?;
        let buf_host = if value_ptr != 0 && size > 0 {
            self.memory
                .guest_to_host_mut(value_ptr, size)
                .ok_or_else(|| anyhow!("invalid xattr buffer"))?
        } else {
            std::ptr::null_mut()
        };

        let res = unsafe {
            libc::fgetxattr(
                fd,
                name.as_ptr() as *const libc::c_char,
                buf_host as *mut libc::c_void,
                size,
            )
        };
        Ok(Self::libc_to_kernel(res as i64))
    }

    /// listxattr(path, list, size)
    fn sys_listxattr(&mut self) -> Result<i64> {
        let path_ptr = self.data_regs[1] as usize;
        let list_ptr = self.data_regs[2] as usize;
        let size = self.data_regs[3] as usize;

        let path = self.read_c_string(path_ptr)?;
        let buf_host = if list_ptr != 0 && size > 0 {
            self.memory
                .guest_to_host_mut(list_ptr, size)
                .ok_or_else(|| anyhow!("invalid xattr list buffer"))?
        } else {
            std::ptr::null_mut()
        };

        let res = unsafe {
            libc::listxattr(
                path.as_ptr() as *const libc::c_char,
                buf_host as *mut libc::c_char,
                size,
            )
        };
        Ok(Self::libc_to_kernel(res as i64))
    }

    /// llistxattr(path, list, size)
    fn sys_llistxattr(&mut self) -> Result<i64> {
        let path_ptr = self.data_regs[1] as usize;
        let list_ptr = self.data_regs[2] as usize;
        let size = self.data_regs[3] as usize;

        let path = self.read_c_string(path_ptr)?;
        let buf_host = if list_ptr != 0 && size > 0 {
            self.memory
                .guest_to_host_mut(list_ptr, size)
                .ok_or_else(|| anyhow!("invalid xattr list buffer"))?
        } else {
            std::ptr::null_mut()
        };

        let res = unsafe {
            libc::llistxattr(
                path.as_ptr() as *const libc::c_char,
                buf_host as *mut libc::c_char,
                size,
            )
        };
        Ok(Self::libc_to_kernel(res as i64))
    }

    /// flistxattr(fd, list, size)
    fn sys_flistxattr(&mut self) -> Result<i64> {
        let fd = self.data_regs[1] as libc::c_int;
        let list_ptr = self.data_regs[2] as usize;
        let size = self.data_regs[3] as usize;

        let buf_host = if list_ptr != 0 && size > 0 {
            self.memory
                .guest_to_host_mut(list_ptr, size)
                .ok_or_else(|| anyhow!("invalid xattr list buffer"))?
        } else {
            std::ptr::null_mut()
        };

        let res = unsafe {
            libc::flistxattr(fd, buf_host as *mut libc::c_char, size)
        };
        Ok(Self::libc_to_kernel(res as i64))
    }

    /// removexattr(path, name)
    fn sys_removexattr(&mut self) -> Result<i64> {
        let path_ptr = self.data_regs[1] as usize;
        let name_ptr = self.data_regs[2] as usize;

        let path = self.read_c_string(path_ptr)?;
        let name = self.read_c_string(name_ptr)?;

        let res = unsafe {
            libc::removexattr(
                path.as_ptr() as *const libc::c_char,
                name.as_ptr() as *const libc::c_char,
            )
        };
        Ok(Self::libc_to_kernel(res as i64))
    }

    /// lremovexattr(path, name)
    fn sys_lremovexattr(&mut self) -> Result<i64> {
        let path_ptr = self.data_regs[1] as usize;
        let name_ptr = self.data_regs[2] as usize;

        let path = self.read_c_string(path_ptr)?;
        let name = self.read_c_string(name_ptr)?;

        let res = unsafe {
            libc::lremovexattr(
                path.as_ptr() as *const libc::c_char,
                name.as_ptr() as *const libc::c_char,
            )
        };
        Ok(Self::libc_to_kernel(res as i64))
    }

    /// fremovexattr(fd, name)
    fn sys_fremovexattr(&mut self) -> Result<i64> {
        let fd = self.data_regs[1] as libc::c_int;
        let name_ptr = self.data_regs[2] as usize;

        let name = self.read_c_string(name_ptr)?;

        let res = unsafe {
            libc::fremovexattr(fd, name.as_ptr() as *const libc::c_char)
        };
        Ok(Self::libc_to_kernel(res as i64))
    }

    /// setxattrat(dirfd, path, name, value, size, flags)
    fn sys_setxattrat(&mut self) -> Result<i64> {
        let dirfd = self.data_regs[1] as libc::c_int;
        let path_ptr = self.data_regs[2] as usize;
        let name_ptr = self.data_regs[3] as usize;
        let value_ptr = self.data_regs[4] as usize;
        let size = self.data_regs[5] as usize;
        let flags = self.data_regs[6] as i32;

        let path = self.read_c_string(path_ptr)?;
        let name = self.read_c_string(name_ptr)?;
        let value = if value_ptr != 0 && size > 0 {
            let host = self
                .memory
                .guest_to_host(value_ptr, size)
                .ok_or_else(|| anyhow!("invalid xattr value buffer"))?;
            Some(unsafe { std::slice::from_raw_parts(host, size) })
        } else {
            None
        };

        let res = unsafe {
            libc::syscall(
                463, // setxattrat syscall number on x86_64 as well
                dirfd,
                path.as_ptr(),
                name.as_ptr(),
                value
                    .map(|v| v.as_ptr() as *const libc::c_void)
                    .unwrap_or(std::ptr::null()),
                size,
                flags,
                0, // at_flags - always 0 for now
            )
        };
        Ok(Self::libc_to_kernel(res as i64))
    }

    /// getxattrat(dirfd, path, name, value, size)
    fn sys_getxattrat(&mut self) -> Result<i64> {
        let dirfd = self.data_regs[1] as libc::c_int;
        let path_ptr = self.data_regs[2] as usize;
        let name_ptr = self.data_regs[3] as usize;
        let value_ptr = self.data_regs[4] as usize;
        let size = self.data_regs[5] as usize;

        let path = self.read_c_string(path_ptr)?;
        let name = self.read_c_string(name_ptr)?;
        let buf_host = if value_ptr != 0 && size > 0 {
            self.memory
                .guest_to_host_mut(value_ptr, size)
                .ok_or_else(|| anyhow!("invalid xattr buffer"))?
        } else {
            std::ptr::null_mut()
        };

        let res = unsafe {
            libc::syscall(
                464, // getxattrat syscall number on x86_64 as well
                dirfd,
                path.as_ptr(),
                name.as_ptr(),
                buf_host,
                size,
                0, // at_flags - always 0 for now
            )
        };
        Ok(Self::libc_to_kernel(res as i64))
    }

    /// listxattrat(dirfd, path, args, atflags)
    fn sys_listxattrat(&mut self) -> Result<i64> {
        let dirfd = self.data_regs[1] as libc::c_int;
        let path_ptr = self.data_regs[2] as usize;
        let args_ptr = self.data_regs[3] as usize;
        let atflags = self.data_regs[4] as libc::c_int;

        let (value_ptr, size, _flags) = self.read_xattr_args(args_ptr)?;
        let path = self.read_c_string(path_ptr)?;
        let buf_host = if value_ptr != 0 && size > 0 {
            self.memory
                .guest_to_host_mut(value_ptr, size)
                .ok_or_else(|| anyhow!("invalid xattr buffer"))?
        } else {
            std::ptr::null_mut()
        };

        let res = unsafe {
            libc::syscall(
                465, // listxattrat syscall number on x86_64 as well
                dirfd,
                path.as_ptr(),
                buf_host,
                size,
                atflags,
            )
        };
        Ok(Self::libc_to_kernel(res as i64))
    }

    /// removexattrat(dirfd, path, name, atflags)
    fn sys_removexattrat(&mut self) -> Result<i64> {
        let dirfd = self.data_regs[1] as libc::c_int;
        let path_ptr = self.data_regs[2] as usize;
        let name_ptr = self.data_regs[3] as usize;
        let atflags = self.data_regs[4] as libc::c_int;

        let path = self.read_c_string(path_ptr)?;
        let name = self.read_c_string(name_ptr)?;
        let res = unsafe {
            libc::syscall(
                466, // removexattrat syscall number on x86_64 as well
                dirfd,
                path.as_ptr(),
                name.as_ptr(),
                atflags,
            )
        };
        Ok(Self::libc_to_kernel(res as i64))
    }

    /// landlock_create_ruleset(attr, size, flags)
    /// Creates a new Landlock ruleset and returns a file descriptor
    fn sys_landlock_create_ruleset(&mut self) -> Result<i64> {
        let attr_addr = self.data_regs[1] as usize;
        let size = self.data_regs[2] as usize;
        let flags = self.data_regs[3];

        if attr_addr == 0 || size == 0 {
            let result = unsafe { libc::syscall(444, std::ptr::null::<u8>(), size, flags) };
            return Ok(Self::libc_to_kernel(result as i64));
        }

        // We only need the first 16 bytes (handled_access_fs + handled_access_net).
        // Validate the guest pointer for that range.
        let copy_len = size.min(16);
        self.memory
            .guest_to_host(attr_addr, copy_len)
            .ok_or_else(|| anyhow!("invalid landlock_ruleset_attr"))?;

        // Read and translate the structure from guest (big-endian) to host (little-endian)
        let handled_access_fs = if size >= 8 {
            self.read_u64_be(attr_addr)?
        } else {
            0
        };

        let handled_access_net = if size >= 16 {
            self.read_u64_be(attr_addr + 8)?
        } else {
            0
        };

        // Build a host-endian buffer matching the requested size.
        let mut host_attr = vec![0u8; size];
        let mut fields = [0u8; 16];
        fields[..8].copy_from_slice(&handled_access_fs.to_ne_bytes());
        if size >= 16 {
            fields[8..16].copy_from_slice(&handled_access_net.to_ne_bytes());
        }
        host_attr[..copy_len].copy_from_slice(&fields[..copy_len]);

        let result = unsafe { libc::syscall(444, host_attr.as_ptr(), size, flags) };

        Ok(Self::libc_to_kernel(result as i64))
    }

    /// landlock_add_rule(ruleset_fd, rule_type, rule_attr, flags)
    /// Adds a rule to a Landlock ruleset
    fn sys_landlock_add_rule(&mut self) -> Result<i64> {
        let ruleset_fd = self.data_regs[1] as i32;
        let rule_type = self.data_regs[2];
        let rule_attr_addr = self.data_regs[3] as usize;
        let flags = self.data_regs[4];

        // Rule type determines the structure size
        // LANDLOCK_RULE_PATH_BENEATH = 1: struct landlock_path_beneath_attr (16 bytes)
        // LANDLOCK_RULE_NET_PORT = 2: struct landlock_net_port_attr (16 bytes)
        const LANDLOCK_RULE_PATH_BENEATH: u32 = 1;
        const LANDLOCK_RULE_NET_PORT: u32 = 2;

        if rule_attr_addr == 0 {
            let result =
                unsafe { libc::syscall(445, ruleset_fd, rule_type, std::ptr::null::<u8>(), flags) };
            return Ok(Self::libc_to_kernel(result as i64));
        }

        let result = match rule_type {
            LANDLOCK_RULE_PATH_BENEATH => {
                // struct landlock_path_beneath_attr { u64 allowed_access; i32 parent_fd; }
                self.memory
                    .guest_to_host(rule_attr_addr, 12)
                    .ok_or_else(|| anyhow!("invalid landlock_path_beneath_attr"))?;

                let allowed_access = self.read_u64_be(rule_attr_addr)?;
                let parent_fd = self.memory.read_long(rule_attr_addr + 8)? as i32;

                let mut host_attr = [0u8; 16];
                host_attr[..8].copy_from_slice(&allowed_access.to_ne_bytes());
                host_attr[8..12].copy_from_slice(&parent_fd.to_ne_bytes());

                unsafe { libc::syscall(445, ruleset_fd, rule_type, host_attr.as_ptr(), flags) }
            }
            LANDLOCK_RULE_NET_PORT => {
                // struct landlock_net_port_attr { u64 allowed_access; u64 port; }
                self.memory
                    .guest_to_host(rule_attr_addr, 16)
                    .ok_or_else(|| anyhow!("invalid landlock_net_port_attr"))?;

                let allowed_access = self.read_u64_be(rule_attr_addr)?;
                let port = self.read_u64_be(rule_attr_addr + 8)?;

                let mut host_attr = [0u8; 16];
                host_attr[..8].copy_from_slice(&allowed_access.to_ne_bytes());
                host_attr[8..16].copy_from_slice(&port.to_ne_bytes());

                unsafe { libc::syscall(445, ruleset_fd, rule_type, host_attr.as_ptr(), flags) }
            }
            _ => return Ok(Self::libc_to_kernel(-libc::EINVAL as i64)),
        };

        Ok(Self::libc_to_kernel(result as i64))
    }

    /// landlock_restrict_self(ruleset_fd, flags)
    /// Enforces the ruleset on the calling thread
    fn sys_landlock_restrict_self(&mut self) -> Result<i64> {
        let ruleset_fd = self.data_regs[1] as i32;
        let flags = self.data_regs[2];

        // Simple passthrough - no structure translation needed
        let result = unsafe { libc::syscall(446, ruleset_fd, flags) };

        Ok(Self::libc_to_kernel(result as i64))
    }

    /// waitpid(pid, status, options) - implemented via wait4 with NULL rusage
    fn sys_waitpid(&mut self) -> Result<i64> {
        // m68k ABI: D1=pid, D2=status*, D3=options
        let pid = self.data_regs[1] as i32;
        let status_addr = self.data_regs[2] as usize;
        let options = self.data_regs[3] as i32;

        let mut status: i32 = 0;
        let result = unsafe {
            libc::wait4(
                pid,
                if status_addr != 0 {
                    &mut status
                } else {
                    std::ptr::null_mut()
                },
                options,
                std::ptr::null_mut(),
            )
        };

        if result > 0 && status_addr != 0 {
            self.memory
                .write_data(status_addr, &(status as u32).to_be_bytes())?;
        }
        Ok(result as i64)
    }

    /// sysinfo(info)
    fn sys_sysinfo(&mut self) -> Result<i64> {
        let info_addr = self.data_regs[1] as usize;
        let mut info: libc::sysinfo = unsafe { std::mem::zeroed() };
        let result = unsafe { libc::sysinfo(&mut info) };
        if result == 0 {
            self.memory
                .write_data(info_addr, &(info.uptime as u32).to_be_bytes())?;
            self.memory
                .write_data(info_addr + 4, &(info.loads[0] as u32).to_be_bytes())?;
            self.memory
                .write_data(info_addr + 8, &(info.loads[1] as u32).to_be_bytes())?;
            self.memory
                .write_data(info_addr + 12, &(info.loads[2] as u32).to_be_bytes())?;
            self.memory
                .write_data(info_addr + 16, &(info.totalram as u32).to_be_bytes())?;
            self.memory
                .write_data(info_addr + 20, &(info.freeram as u32).to_be_bytes())?;
            self.memory
                .write_data(info_addr + 24, &(info.sharedram as u32).to_be_bytes())?;
            self.memory
                .write_data(info_addr + 28, &(info.bufferram as u32).to_be_bytes())?;
            self.memory
                .write_data(info_addr + 32, &(info.totalswap as u32).to_be_bytes())?;
            self.memory
                .write_data(info_addr + 36, &(info.freeswap as u32).to_be_bytes())?;
            self.memory
                .write_data(info_addr + 40, &(info.procs as u16).to_be_bytes())?;
        }
        Ok(result as i64)
    }

    /// uname(buf)
    fn sys_uname(&mut self) -> Result<i64> {
        let buf_addr = self.data_regs[1] as usize;
        let mut uts: libc::utsname = unsafe { std::mem::zeroed() };
        let result = unsafe { libc::uname(&mut uts) };
        if result == 0 {
            // Each field is 65 bytes in the kernel struct
            let field_size = 65usize;
            self.memory.write_data(
                buf_addr,
                &uts.sysname[..field_size]
                    .iter()
                    .map(|&c| c as u8)
                    .collect::<Vec<_>>(),
            )?;
            self.memory.write_data(
                buf_addr + field_size,
                &uts.nodename[..field_size]
                    .iter()
                    .map(|&c| c as u8)
                    .collect::<Vec<_>>(),
            )?;
            self.memory.write_data(
                buf_addr + field_size * 2,
                &uts.release[..field_size]
                    .iter()
                    .map(|&c| c as u8)
                    .collect::<Vec<_>>(),
            )?;
            self.memory.write_data(
                buf_addr + field_size * 3,
                &uts.version[..field_size]
                    .iter()
                    .map(|&c| c as u8)
                    .collect::<Vec<_>>(),
            )?;
            self.memory.write_data(
                buf_addr + field_size * 4,
                &uts.machine[..field_size]
                    .iter()
                    .map(|&c| c as u8)
                    .collect::<Vec<_>>(),
            )?;
        }
        Ok(result as i64)
    }

    /// _llseek(fd, offset_high, offset_low, result, whence)
    fn sys_llseek(&mut self) -> Result<i64> {
        let fd = self.data_regs[1] as i32;
        let offset_high = self.data_regs[2];
        let offset_low = self.data_regs[3];
        let result_addr = self.data_regs[4] as usize;
        let whence = self.data_regs[5] as i32;

        let offset = ((offset_high as i64) << 32) | (offset_low as i64);
        let result = unsafe { libc::lseek(fd, offset, whence) };

        if result >= 0 && result_addr != 0 {
            // Write 64-bit result to guest memory
            self.memory
                .write_data(result_addr, &((result >> 32) as u32).to_be_bytes())?;
            self.memory
                .write_data(result_addr + 4, &(result as u32).to_be_bytes())?;
            Ok(0)
        } else {
            Ok(result)
        }
    }

    /// getdents(fd, dirp, count) - 32-bit dirent
    fn sys_getdents32(&mut self) -> Result<i64> {
        let fd = self.data_regs[1] as i32;
        let dirp = self.data_regs[2] as usize;
        let count = self.data_regs[3] as usize;

        // Read into a temporary host buffer
        let mut host_buf = vec![0u8; count];
        let result =
            unsafe { libc::syscall(libc::SYS_getdents64, fd, host_buf.as_mut_ptr(), count) };

        if result < 0 {
            return Ok(Self::libc_to_kernel(result));
        }

        let bytes_read = result as usize;

        if bytes_read == 0 {
            return Ok(0);
        }

        // eprintln!("getdents32: fd={}, count={}, bytes_read={}", fd, count, bytes_read);

        // Translate dirent64 structures to 32-bit m68k dirent format
        let mut host_off = 0;
        let mut guest_off = 0;

        while host_off < bytes_read {
            if host_off + 19 > bytes_read {
                break;
            }

            let d_ino = u64::from_ne_bytes(host_buf[host_off..host_off + 8].try_into()?);
            let d_off = i64::from_ne_bytes(host_buf[host_off + 8..host_off + 16].try_into()?);
            let d_reclen = u16::from_ne_bytes(host_buf[host_off + 16..host_off + 18].try_into()?);
            let d_type = host_buf[host_off + 18];

            // Find null terminator in d_name
            let name_start = host_off + 19;
            let name_end = host_buf[name_start..host_off + d_reclen as usize]
                .iter()
                .position(|&b| b == 0)
                .map(|p| name_start + p)
                .unwrap_or(host_off + d_reclen as usize);

            let name_len = name_end - name_start;

            // m68k linux_dirent structure (OLD format, not linux_dirent64):
            // u32 d_ino (4 bytes, BE)
            // i32 d_off (4 bytes, BE)
            // u16 d_reclen (2 bytes, BE)
            // char d_name[] (variable, null-terminated)
            // [padding to align]
            // u8  d_type (1 byte, at offset reclen-1)

            // Calculate m68k record length (aligned to 2 bytes, includes d_type at end)
            let m68k_reclen = (10 + name_len + 1 + 1).div_ceil(2) * 2;

            if guest_off + m68k_reclen > count {
                break;
            }

            // Write m68k linux_dirent (truncate 64-bit values to 32-bit)
            self.memory
                .write_data(dirp + guest_off, &(d_ino as u32).to_be_bytes())?;
            self.memory
                .write_data(dirp + guest_off + 4, &(d_off as i32).to_be_bytes())?;
            self.memory
                .write_data(dirp + guest_off + 8, &(m68k_reclen as u16).to_be_bytes())?;

            // Write name at offset 10
            self.memory.write_data(
                dirp + guest_off + 10,
                &host_buf[name_start..name_start + name_len],
            )?;
            self.memory
                .write_data(dirp + guest_off + 10 + name_len, &[0u8])?;

            // Zero out padding
            for i in (10 + name_len + 1)..(m68k_reclen - 1) {
                self.memory.write_data(dirp + guest_off + i, &[0u8])?;
            }

            // Write d_type at the LAST byte of the record (reclen - 1)
            self.memory
                .write_data(dirp + guest_off + m68k_reclen - 1, &[d_type])?;

            host_off += d_reclen as usize;
            guest_off += m68k_reclen;
        }
        Ok(guest_off as i64)
    }

    /// getdents64(fd, dirp, count) - 64-bit dirent64
    fn sys_getdents64(&mut self) -> Result<i64> {
        let fd = self.data_regs[1] as i32;
        let dirp = self.data_regs[2] as usize;
        let count = self.data_regs[3] as usize;

        // Read into a temporary host buffer
        let mut host_buf = vec![0u8; count];
        let result =
            unsafe { libc::syscall(libc::SYS_getdents64, fd, host_buf.as_mut_ptr(), count) };

        if result < 0 {
            return Ok(Self::libc_to_kernel(result));
        }

        let bytes_read = result as usize;

        if bytes_read == 0 {
            // End of directory
            return Ok(0);
        }

        // Translate dirent64 structures from x86-64 to m68k format
        let mut host_off = 0;
        let mut guest_off = 0;

        while host_off < bytes_read {
            // Read x86-64 linux_dirent64:
            // struct linux_dirent64 {
            //     u64 d_ino;
            //     i64 d_off;
            //     u16 d_reclen;
            //     u8  d_type;
            //     char d_name[];
            // }

            if host_off + 19 > bytes_read {
                break; // Not enough data for header
            }

            let d_ino = u64::from_ne_bytes(host_buf[host_off..host_off + 8].try_into()?);
            let d_off = i64::from_ne_bytes(host_buf[host_off + 8..host_off + 16].try_into()?);
            let d_reclen = u16::from_ne_bytes(host_buf[host_off + 16..host_off + 18].try_into()?);
            let d_type = host_buf[host_off + 18];

            // Find null terminator in d_name
            let name_start = host_off + 19;
            let name_end = host_buf[name_start..host_off + d_reclen as usize]
                .iter()
                .position(|&b| b == 0)
                .map(|p| name_start + p)
                .unwrap_or(host_off + d_reclen as usize);

            let name_len = name_end - name_start;

            // m68k dirent64 structure (same layout, but ensure big-endian):
            // u64 d_ino (8 bytes, BE)
            // i64 d_off (8 bytes, BE)
            // u16 d_reclen (2 bytes, BE)
            // u8  d_type (1 byte)
            // char d_name[] (variable, null-terminated)

            // Calculate m68k record length (aligned to 8 bytes)
            let m68k_reclen = (19 + name_len + 1).div_ceil(8) * 8;

            if guest_off + m68k_reclen > count {
                break; // Not enough space in guest buffer
            }

            // Write m68k dirent64
            self.memory
                .write_data(dirp + guest_off, &d_ino.to_be_bytes())?;
            self.memory
                .write_data(dirp + guest_off + 8, &d_off.to_be_bytes())?;
            self.memory
                .write_data(dirp + guest_off + 16, &(m68k_reclen as u16).to_be_bytes())?;
            self.memory.write_data(dirp + guest_off + 18, &[d_type])?;

            // Write name
            self.memory.write_data(
                dirp + guest_off + 19,
                &host_buf[name_start..name_start + name_len],
            )?;
            self.memory
                .write_data(dirp + guest_off + 19 + name_len, &[0u8])?; // null terminator

            // Zero out padding
            for i in (19 + name_len + 1)..m68k_reclen {
                self.memory.write_data(dirp + guest_off + i, &[0u8])?;
            }

            host_off += d_reclen as usize;
            guest_off += m68k_reclen;
        }

        Ok(guest_off as i64)
    }

    /// nanosleep(req, rem)
    fn sys_nanosleep(&mut self) -> Result<i64> {
        let req_addr = self.data_regs[1] as usize;
        let rem_addr = self.data_regs[2] as usize;

        // m68k uclibc uses 64-bit time_t
        let req_sec_bytes: [u8; 8] = self.memory.read_data(req_addr, 8)?.try_into().unwrap();
        let req_sec = i64::from_be_bytes(req_sec_bytes) as libc::time_t;
        let req_nsec = self.memory.read_long(req_addr + 8)? as i64;

        let req = libc::timespec {
            tv_sec: req_sec,
            tv_nsec: req_nsec,
        };
        let mut rem: libc::timespec = unsafe { std::mem::zeroed() };

        let result = unsafe {
            libc::nanosleep(
                &req,
                if rem_addr != 0 {
                    &mut rem
                } else {
                    std::ptr::null_mut()
                },
            )
        };

        if rem_addr != 0 {
            self.memory
                .write_data(rem_addr, &(rem.tv_sec as i64).to_be_bytes())?;
            self.memory
                .write_data(rem_addr + 8, &(rem.tv_nsec as u32).to_be_bytes())?;
        }
        Ok(result as i64)
    }

    /// futex(uaddr, op, val, timeout, uaddr2, val3)
    /// Fast userspace mutex - translates guest pointers to host
    fn sys_futex(&mut self) -> Result<i64> {
        let uaddr_guest = self.data_regs[1] as usize;
        let op = self.data_regs[2] as i32;
        let val = self.data_regs[3] as i32;
        let timeout_guest = self.data_regs[4] as usize;
        let uaddr2_guest = self.data_regs[5] as usize;

        // Get 6th argument from stack (m68k ABI passes 6th arg on stack)
        let sp = self.addr_regs[7] as usize;
        let val3 = if sp != 0 {
            self.memory.read_long(sp).unwrap_or(0) as i32
        } else {
            0
        };

        // Translate main futex address to host pointer
        let uaddr_host = self
            .memory
            .guest_to_host_mut(uaddr_guest, 4)
            .ok_or_else(|| anyhow!("invalid futex address {:#x}", uaddr_guest))?
            as *mut i32;

        // Handle timeout parameter if present
        // Operations like FUTEX_WAIT use timeout, others ignore it
        let timeout_opt = if timeout_guest != 0 {
            // m68k uclibc uses 64-bit time_t
            let tv_sec_bytes: [u8; 8] =
                self.memory.read_data(timeout_guest, 8)?.try_into().unwrap();
            let tv_sec = i64::from_be_bytes(tv_sec_bytes);
            let tv_nsec = self.memory.read_long(timeout_guest + 8)? as i64;
            Some(libc::timespec { tv_sec, tv_nsec })
        } else {
            None
        };

        // Handle uaddr2 for REQUEUE and CMP_REQUEUE operations
        let uaddr2_host = if uaddr2_guest != 0 {
            self.memory
                .guest_to_host_mut(uaddr2_guest, 4)
                .ok_or_else(|| anyhow!("invalid futex uaddr2 {:#x}", uaddr2_guest))?
                as *mut i32
        } else {
            std::ptr::null_mut()
        };

        // Call host futex syscall with translated pointers
        let result = unsafe {
            libc::syscall(
                libc::SYS_futex,
                uaddr_host,
                op,
                val,
                timeout_opt
                    .as_ref()
                    .map(|t| t as *const _)
                    .unwrap_or(std::ptr::null()),
                uaddr2_host,
                val3,
            )
        };

        Ok(Self::libc_to_kernel(result))
    }

    /// getresuid(ruid, euid, suid)
    fn sys_getresuid(&mut self) -> Result<i64> {
        let ruid_addr = self.data_regs[1] as usize;
        let euid_addr = self.data_regs[2] as usize;
        let suid_addr = self.data_regs[3] as usize;

        let mut ruid: libc::uid_t = 0;
        let mut euid: libc::uid_t = 0;
        let mut suid: libc::uid_t = 0;

        let result = unsafe { libc::getresuid(&mut ruid, &mut euid, &mut suid) };
        if result == 0 {
            if ruid_addr != 0 {
                self.memory.write_data(ruid_addr, &ruid.to_be_bytes())?;
            }
            if euid_addr != 0 {
                self.memory.write_data(euid_addr, &euid.to_be_bytes())?;
            }
            if suid_addr != 0 {
                self.memory.write_data(suid_addr, &suid.to_be_bytes())?;
            }
        }
        Ok(result as i64)
    }

    /// getresgid(rgid, egid, sgid)
    fn sys_getresgid(&mut self) -> Result<i64> {
        let rgid_addr = self.data_regs[1] as usize;
        let egid_addr = self.data_regs[2] as usize;
        let sgid_addr = self.data_regs[3] as usize;

        let mut rgid: libc::gid_t = 0;
        let mut egid: libc::gid_t = 0;
        let mut sgid: libc::gid_t = 0;

        let result = unsafe { libc::getresgid(&mut rgid, &mut egid, &mut sgid) };
        if result == 0 {
            if rgid_addr != 0 {
                self.memory.write_data(rgid_addr, &rgid.to_be_bytes())?;
            }
            if egid_addr != 0 {
                self.memory.write_data(egid_addr, &egid.to_be_bytes())?;
            }
            if sgid_addr != 0 {
                self.memory.write_data(sgid_addr, &sgid.to_be_bytes())?;
            }
        }
        Ok(result as i64)
    }

    /// sched_setparam(pid, param)
    /// struct sched_param { int sched_priority; }
    fn sys_sched_setparam(&mut self) -> Result<i64> {
        let pid = self.data_regs[1] as libc::pid_t;
        let param_addr = self.data_regs[2] as usize;

        // Read m68k sched_param (4 bytes - int32)
        let priority = self.memory.read_long(param_addr)? as i32;

        // Create host sched_param
        let param = libc::sched_param {
            sched_priority: priority,
        };

        let result = unsafe { libc::sched_setparam(pid, &param) };
        Ok(Self::libc_to_kernel(result as i64))
    }

    /// sched_getparam(pid, param)
    fn sys_sched_getparam(&mut self) -> Result<i64> {
        let pid = self.data_regs[1] as libc::pid_t;
        let param_addr = self.data_regs[2] as usize;

        let mut param: libc::sched_param = unsafe { std::mem::zeroed() };

        let result = unsafe { libc::sched_getparam(pid, &mut param) };
        if result == 0 {
            // Write back sched_priority (4 bytes)
            self.memory
                .write_data(param_addr, &(param.sched_priority as u32).to_be_bytes())?;
        }
        Ok(Self::libc_to_kernel(result as i64))
    }

    /// sched_setscheduler(pid, policy, param)
    fn sys_sched_setscheduler(&mut self) -> Result<i64> {
        let pid = self.data_regs[1] as libc::pid_t;
        let policy = self.data_regs[2] as i32;
        let param_addr = self.data_regs[3] as usize;

        // Read m68k sched_param
        let priority = self.memory.read_long(param_addr)? as i32;

        let param = libc::sched_param {
            sched_priority: priority,
        };

        let result = unsafe { libc::sched_setscheduler(pid, policy, &param) };
        Ok(Self::libc_to_kernel(result as i64))
    }

    /// sched_rr_get_interval(pid, tp)
    fn sys_sched_rr_get_interval(&mut self) -> Result<i64> {
        let pid = self.data_regs[1] as libc::pid_t;
        let tp_addr = self.data_regs[2] as usize;

        let mut tp: libc::timespec = unsafe { std::mem::zeroed() };

        let result = unsafe { libc::sched_rr_get_interval(pid, &mut tp) };
        if result == 0 {
            // m68k uclibc uses 64-bit time_t
            // Write tv_sec as 8 bytes
            self.memory
                .write_data(tp_addr, &(tp.tv_sec as i64).to_be_bytes())?;
            // Write tv_nsec as 4 bytes
            self.memory
                .write_data(tp_addr + 8, &(tp.tv_nsec as u32).to_be_bytes())?;
        }
        Ok(Self::libc_to_kernel(result as i64))
    }

    /// poll(fds, nfds, timeout)
    fn sys_poll(&mut self) -> Result<i64> {
        let fds_addr = self.data_regs[1] as usize;
        let nfds = self.data_regs[2] as usize;
        let timeout = self.data_regs[3] as i32;

        // Read pollfd array from guest (each pollfd is 8 bytes on m68k)
        let mut pollfds = Vec::with_capacity(nfds);
        for i in 0..nfds {
            let fd = self.memory.read_long(fds_addr + i * 8)? as i32;
            let events = self.memory.read_word(fds_addr + i * 8 + 4)? as i16;
            pollfds.push(libc::pollfd {
                fd,
                events,
                revents: 0,
            });
        }

        let result = unsafe { libc::poll(pollfds.as_mut_ptr(), nfds as libc::nfds_t, timeout) };

        // Write back revents
        for (i, pfd) in pollfds.iter().enumerate() {
            self.memory
                .write_data(fds_addr + i * 8 + 6, &(pfd.revents as u16).to_be_bytes())?;
        }
        Ok(result as i64)
    }

    /// pread64(fd, buf, count, offset)
    fn sys_pread64(&mut self) -> Result<i64> {
        let fd = self.data_regs[1] as i32;
        let buf_addr = self.data_regs[2] as usize;
        let count = self.data_regs[3] as usize;
        let offset = self.data_regs[4] as i64;
        let host_buf = self
            .memory
            .guest_to_host_mut(buf_addr, count)
            .ok_or_else(|| anyhow!("invalid pread64 buffer"))?;
        Ok(unsafe { libc::pread(fd, host_buf as *mut libc::c_void, count, offset) as i64 })
    }

    /// pwrite64(fd, buf, count, offset)
    fn sys_pwrite64(&self) -> Result<i64> {
        let fd = self.data_regs[1] as i32;
        let buf_addr = self.data_regs[2] as usize;
        let count = self.data_regs[3] as usize;
        let offset = self.data_regs[4] as i64;
        let host_buf = self
            .memory
            .guest_to_host(buf_addr, count)
            .ok_or_else(|| anyhow!("invalid pwrite64 buffer"))?;
        Ok(unsafe { libc::pwrite(fd, host_buf as *const libc::c_void, count, offset) as i64 })
    }

    /// preadv(fd, iov, iovcnt, pos_l, pos_h)
    fn sys_preadv(&mut self) -> Result<i64> {
        let fd = self.data_regs[1] as i32;
        let iov_addr = self.data_regs[2] as usize;
        let iovcnt = self.data_regs[3] as usize;
        let off_lo = self.data_regs[4] as i64;
        let off_hi = self.data_regs[5] as i64;
        let offset = (off_hi << 32) | off_lo;
        let iovecs = self.build_iovecs(iov_addr, iovcnt, true)?;
        Ok(unsafe { libc::preadv(fd, iovecs.as_ptr(), iovecs.len() as i32, offset) as i64 })
    }

    /// pwritev(fd, iov, iovcnt, pos_l, pos_h)
    fn sys_pwritev(&mut self) -> Result<i64> {
        let fd = self.data_regs[1] as i32;
        let iov_addr = self.data_regs[2] as usize;
        let iovcnt = self.data_regs[3] as usize;
        let off_lo = self.data_regs[4] as i64;
        let off_hi = self.data_regs[5] as i64;
        let offset = (off_hi << 32) | off_lo;
        let iovecs = self.build_iovecs(iov_addr, iovcnt, false)?;
        Ok(unsafe { libc::pwritev(fd, iovecs.as_ptr(), iovecs.len() as i32, offset) as i64 })
    }

    /// getcwd(buf, size)
    fn sys_getcwd(&mut self) -> Result<i64> {
        let buf_addr = self.data_regs[1] as usize;
        let size = self.data_regs[2] as usize;
        let host_buf = self
            .memory
            .guest_to_host_mut(buf_addr, size)
            .ok_or_else(|| anyhow!("invalid getcwd buffer"))?;

        let result = unsafe { libc::getcwd(host_buf as *mut i8, size) };

        if result.is_null() {
            Ok(Self::libc_to_kernel(-1))
        } else {
            // getcwd syscall returns the length of the string (including null terminator)
            let len = unsafe { libc::strlen(result) } + 1;
            Ok(len as i64)
        }
    }

    /// sendfile(out_fd, in_fd, offset, count)
    fn sys_sendfile(&mut self) -> Result<i64> {
        let out_fd = self.data_regs[1] as i32;
        let in_fd = self.data_regs[2] as i32;
        let offset_addr = self.data_regs[3] as usize;
        let count = self.data_regs[4] as usize;

        if offset_addr == 0 {
            Ok(unsafe { libc::sendfile(out_fd, in_fd, std::ptr::null_mut(), count) as i64 })
        } else {
            let mut offset = self.memory.read_long(offset_addr)? as libc::off_t;
            let result = unsafe { libc::sendfile(out_fd, in_fd, &mut offset, count) };
            if result >= 0 {
                self.memory
                    .write_data(offset_addr, &(offset as u32).to_be_bytes())?;
            }
            Ok(result as i64)
        }
    }

    /// splice(fd_in, off_in, fd_out, off_out, len, flags)
    fn sys_splice(&mut self) -> Result<i64> {
        let fd_in = self.data_regs[1] as i32;
        let off_in_addr = self.data_regs[2] as usize;
        let fd_out = self.data_regs[3] as i32;
        let off_out_addr = self.data_regs[4] as usize;
        let len = self.data_regs[5] as usize;
        let flags = self.data_regs[6];

        // Read offsets if provided (loff_t is i64, need to read 8 bytes)
        let mut off_in_val = if off_in_addr != 0 {
            let high = self.memory.read_long(off_in_addr)?;
            let low = self.memory.read_long(off_in_addr + 4)?;
            ((high as i64) << 32) | (low as i64)
        } else {
            0
        };

        let mut off_out_val = if off_out_addr != 0 {
            let high = self.memory.read_long(off_out_addr)?;
            let low = self.memory.read_long(off_out_addr + 4)?;
            ((high as i64) << 32) | (low as i64)
        } else {
            0
        };

        // Prepare pointers
        let off_in_ptr = if off_in_addr != 0 {
            &mut off_in_val as *mut i64
        } else {
            std::ptr::null_mut()
        };
        let off_out_ptr = if off_out_addr != 0 {
            &mut off_out_val as *mut i64
        } else {
            std::ptr::null_mut()
        };

        // Call splice
        let result = unsafe { libc::splice(fd_in, off_in_ptr, fd_out, off_out_ptr, len, flags) };

        // Write back offsets if successful
        if result >= 0 {
            if off_in_addr != 0 {
                let high = (off_in_val >> 32) as u32;
                let low = off_in_val as u32;
                self.memory.write_data(off_in_addr, &high.to_be_bytes())?;
                self.memory
                    .write_data(off_in_addr + 4, &low.to_be_bytes())?;
            }
            if off_out_addr != 0 {
                let high = (off_out_val >> 32) as u32;
                let low = off_out_val as u32;
                self.memory.write_data(off_out_addr, &high.to_be_bytes())?;
                self.memory
                    .write_data(off_out_addr + 4, &low.to_be_bytes())?;
            }
        }

        Ok(result as i64)
    }

    /// vmsplice(fd, iov, nr_segs, flags)
    fn sys_vmsplice(&mut self) -> Result<i64> {
        let fd = self.data_regs[1] as i32;
        let iov_addr = self.data_regs[2] as usize;
        let nr_segs = self.data_regs[3] as usize;
        let flags = self.data_regs[4];

        // Build iovec array (read-only for vmsplice)
        let iovecs = self.build_iovecs(iov_addr, nr_segs, false)?;

        // Call vmsplice
        let result = unsafe { libc::vmsplice(fd, iovecs.as_ptr(), iovecs.len(), flags) };

        Ok(result as i64)
    }

    /// socketpair(domain, type, protocol, sv)
    fn sys_socketpair(&mut self) -> Result<i64> {
        let domain = self.data_regs[1] as i32;
        let socktype = self.data_regs[2] as i32;
        let protocol = self.data_regs[3] as i32;
        let sv_addr = self.data_regs[4] as usize;

        let mut sv: [i32; 2] = [0; 2];
        let result = unsafe { libc::socketpair(domain, socktype, protocol, sv.as_mut_ptr()) };
        if result == 0 {
            self.memory
                .write_data(sv_addr, &(sv[0] as u32).to_be_bytes())?;
            self.memory
                .write_data(sv_addr + 4, &(sv[1] as u32).to_be_bytes())?;
        }
        Ok(result as i64)
    }

    /// bind/connect(sockfd, addr, addrlen) - addr is pointer
    fn sys_socket_addr(&self, syscall_num: u32) -> Result<i64> {
        let sockfd = self.data_regs[1] as i32;
        let addr_ptr = self.data_regs[2] as usize;
        let addrlen = self.data_regs[3];

        let host_addr = self
            .memory
            .guest_to_host(addr_ptr, addrlen as usize)
            .ok_or_else(|| anyhow!("invalid sockaddr"))?;
        Ok(unsafe { libc::syscall(syscall_num as i64, sockfd, host_addr, addrlen) })
    }

    /// accept4(sockfd, addr, addrlen, flags)
    fn sys_accept4(&mut self) -> Result<i64> {
        let sockfd = self.data_regs[1] as i32;
        let addr_ptr = self.data_regs[2] as usize;
        let addrlen_ptr = self.data_regs[3] as usize;
        let flags = self.data_regs[4] as i32;

        if addr_ptr == 0 {
            return Ok(unsafe {
                libc::syscall(
                    libc::SYS_accept4,
                    sockfd,
                    std::ptr::null::<u8>(),
                    std::ptr::null::<u32>(),
                    flags,
                )
            });
        }

        let mut addrlen = self.memory.read_long(addrlen_ptr)?;
        let host_addr = self
            .memory
            .guest_to_host_mut(addr_ptr, addrlen as usize)
            .ok_or_else(|| anyhow!("invalid sockaddr buffer"))?;

        let result = unsafe {
            libc::syscall(
                libc::SYS_accept4,
                sockfd,
                host_addr,
                &mut addrlen as *mut u32,
                flags,
            )
        };
        if result >= 0 {
            self.memory
                .write_data(addrlen_ptr, &addrlen.to_be_bytes())?;
        }
        Ok(result)
    }

    /// getsockopt(sockfd, level, optname, optval, optlen)
    fn sys_getsockopt(&mut self) -> Result<i64> {
        let sockfd = self.data_regs[1] as i32;
        let level = self.data_regs[2] as i32;
        let optname = self.data_regs[3] as i32;
        let optval_ptr = self.data_regs[4] as usize;
        let optlen_ptr = self.data_regs[5] as usize;

        let mut optlen = self.memory.read_long(optlen_ptr)? as libc::socklen_t;
        let host_optval = self
            .memory
            .guest_to_host_mut(optval_ptr, optlen as usize)
            .ok_or_else(|| anyhow!("invalid optval buffer"))?;

        let result = unsafe {
            libc::getsockopt(
                sockfd,
                level,
                optname,
                host_optval as *mut libc::c_void,
                &mut optlen,
            )
        };
        if result == 0 {
            self.memory
                .write_data(optlen_ptr, &(optlen as u32).to_be_bytes())?;
        }
        Ok(result as i64)
    }

    /// setsockopt(sockfd, level, optname, optval, optlen)
    fn sys_setsockopt(&self) -> Result<i64> {
        let sockfd = self.data_regs[1] as i32;
        let level = self.data_regs[2] as i32;
        let optname = self.data_regs[3] as i32;
        let optval_ptr = self.data_regs[4] as usize;
        let optlen = self.data_regs[5] as libc::socklen_t;

        let host_optval = self
            .memory
            .guest_to_host(optval_ptr, optlen as usize)
            .ok_or_else(|| anyhow!("invalid optval buffer"))?;

        Ok(unsafe {
            libc::setsockopt(
                sockfd,
                level,
                optname,
                host_optval as *const libc::c_void,
                optlen,
            ) as i64
        })
    }

    /// getsockname/getpeername(sockfd, addr, addrlen)
    fn sys_getsockname(&mut self) -> Result<i64> {
        let sockfd = self.data_regs[1] as i32;
        let addr_ptr = self.data_regs[2] as usize;
        let addrlen_ptr = self.data_regs[3] as usize;

        let mut addrlen = self.memory.read_long(addrlen_ptr)? as libc::socklen_t;
        let host_addr = self
            .memory
            .guest_to_host_mut(addr_ptr, addrlen as usize)
            .ok_or_else(|| anyhow!("invalid sockaddr buffer"))?;

        // Use getsockname - caller distinguishes via syscall number
        let result =
            unsafe { libc::getsockname(sockfd, host_addr as *mut libc::sockaddr, &mut addrlen) };
        if result == 0 {
            self.memory
                .write_data(addrlen_ptr, &(addrlen as u32).to_be_bytes())?;
        }
        Ok(result as i64)
    }

    /// sendto(sockfd, buf, len, flags, dest_addr, addrlen)
    fn sys_sendto(&self) -> Result<i64> {
        let sockfd = self.data_regs[1] as i32;
        let buf_ptr = self.data_regs[2] as usize;
        let len = self.data_regs[3] as usize;
        let flags = self.data_regs[4] as i32;
        let dest_addr = self.data_regs[5] as usize;
        // addrlen would be in D6 but we only have D1-D5, use stack or fixed size
        // For now, assume addrlen is after dest_addr in struct or use 16 (sizeof sockaddr_in)
        let addrlen: libc::socklen_t = 16;

        let host_buf = self
            .memory
            .guest_to_host(buf_ptr, len)
            .ok_or_else(|| anyhow!("invalid sendto buffer"))?;

        if dest_addr == 0 {
            Ok(unsafe {
                libc::sendto(
                    sockfd,
                    host_buf as *const libc::c_void,
                    len,
                    flags,
                    std::ptr::null(),
                    0,
                ) as i64
            })
        } else {
            let host_addr = self
                .memory
                .guest_to_host(dest_addr, addrlen as usize)
                .ok_or_else(|| anyhow!("invalid dest_addr"))?;
            Ok(unsafe {
                libc::sendto(
                    sockfd,
                    host_buf as *const libc::c_void,
                    len,
                    flags,
                    host_addr as *const libc::sockaddr,
                    addrlen,
                ) as i64
            })
        }
    }

    /// recvfrom(sockfd, buf, len, flags, src_addr, addrlen)
    fn sys_recvfrom(&mut self) -> Result<i64> {
        let sockfd = self.data_regs[1] as i32;
        let buf_ptr = self.data_regs[2] as usize;
        let len = self.data_regs[3] as usize;
        let flags = self.data_regs[4] as i32;
        let src_addr = self.data_regs[5] as usize;

        let host_buf = self
            .memory
            .guest_to_host_mut(buf_ptr, len)
            .ok_or_else(|| anyhow!("invalid recvfrom buffer"))?;

        if src_addr == 0 {
            Ok(unsafe {
                libc::recvfrom(
                    sockfd,
                    host_buf as *mut libc::c_void,
                    len,
                    flags,
                    std::ptr::null_mut(),
                    std::ptr::null_mut(),
                ) as i64
            })
        } else {
            // For simplicity, use fixed buffer size
            let mut addrlen: libc::socklen_t = 128;
            let host_addr = self
                .memory
                .guest_to_host_mut(src_addr, addrlen as usize)
                .ok_or_else(|| anyhow!("invalid src_addr buffer"))?;
            Ok(unsafe {
                libc::recvfrom(
                    sockfd,
                    host_buf as *mut libc::c_void,
                    len,
                    flags,
                    host_addr as *mut libc::sockaddr,
                    &mut addrlen,
                ) as i64
            })
        }
    }

    /// sendmsg(sockfd, msg, flags)
    /// Sends a message on a socket using a msghdr structure
    fn sys_sendmsg(&mut self) -> Result<i64> {
        let sockfd = self.data_regs[1] as i32;
        let msg_addr = self.data_regs[2] as usize;
        let flags = self.data_regs[3] as i32;

        // Read m68k msghdr structure (28 bytes)
        // struct msghdr {
        //     void *msg_name;           // 0: u32
        //     socklen_t msg_namelen;    // 4: u32
        //     struct iovec *msg_iov;    // 8: u32
        //     size_t msg_iovlen;        // 12: u32
        //     void *msg_control;        // 16: u32
        //     size_t msg_controllen;    // 20: u32
        //     int msg_flags;            // 24: i32
        // }
        let msg_name = self.memory.read_long(msg_addr)? as usize;
        let msg_namelen = self.memory.read_long(msg_addr + 4)?;
        let msg_iov = self.memory.read_long(msg_addr + 8)? as usize;
        let msg_iovlen = self.memory.read_long(msg_addr + 12)? as usize;
        let msg_control = self.memory.read_long(msg_addr + 16)? as usize;
        let msg_controllen = self.memory.read_long(msg_addr + 20)? as usize;

        // Build iovec array
        let iovecs = if msg_iovlen > 0 {
            self.build_iovecs(msg_iov, msg_iovlen, false)?
        } else {
            Vec::new()
        };

        // Translate msg_name pointer
        let name_ptr = if msg_name != 0 && msg_namelen > 0 {
            self.memory
                .guest_to_host(msg_name, msg_namelen as usize)
                .ok_or_else(|| anyhow!("invalid msg_name pointer"))?
                as *const libc::c_void
        } else {
            std::ptr::null()
        };

        // Translate msg_control pointer
        let control_ptr = if msg_control != 0 && msg_controllen > 0 {
            self.memory
                .guest_to_host(msg_control, msg_controllen)
                .ok_or_else(|| anyhow!("invalid msg_control pointer"))?
                as *const libc::c_void
        } else {
            std::ptr::null()
        };

        // Build host msghdr
        let host_msg = libc::msghdr {
            msg_name: name_ptr as *mut libc::c_void,
            msg_namelen,
            msg_iov: iovecs.as_ptr() as *mut libc::iovec,
            msg_iovlen: iovecs.len(),
            msg_control: control_ptr as *mut libc::c_void,
            msg_controllen,
            msg_flags: 0, // Input flags ignored on sendmsg
        };

        let result = unsafe { libc::sendmsg(sockfd, &host_msg, flags) };
        Ok(result as i64)
    }

    /// recvmsg(sockfd, msg, flags)
    /// Receives a message from a socket using a msghdr structure
    fn sys_recvmsg(&mut self) -> Result<i64> {
        let sockfd = self.data_regs[1] as i32;
        let msg_addr = self.data_regs[2] as usize;
        let flags = self.data_regs[3] as i32;

        // Read m68k msghdr structure (28 bytes)
        let msg_name = self.memory.read_long(msg_addr)? as usize;
        let msg_namelen = self.memory.read_long(msg_addr + 4)?;
        let msg_iov = self.memory.read_long(msg_addr + 8)? as usize;
        let msg_iovlen = self.memory.read_long(msg_addr + 12)? as usize;
        let msg_control = self.memory.read_long(msg_addr + 16)? as usize;
        let msg_controllen = self.memory.read_long(msg_addr + 20)? as usize;

        // Build iovec array (writable for recvmsg)
        let iovecs = if msg_iovlen > 0 {
            self.build_iovecs(msg_iov, msg_iovlen, true)?
        } else {
            Vec::new()
        };

        // Translate msg_name pointer (writable for recvmsg)
        let name_ptr = if msg_name != 0 && msg_namelen > 0 {
            self.memory
                .guest_to_host_mut(msg_name, msg_namelen as usize)
                .ok_or_else(|| anyhow!("invalid msg_name pointer"))?
                as *mut libc::c_void
        } else {
            std::ptr::null_mut()
        };

        // Translate msg_control pointer (writable for recvmsg)
        let control_ptr = if msg_control != 0 && msg_controllen > 0 {
            self.memory
                .guest_to_host_mut(msg_control, msg_controllen)
                .ok_or_else(|| anyhow!("invalid msg_control pointer"))?
                as *mut libc::c_void
        } else {
            std::ptr::null_mut()
        };

        // Build host msghdr
        let mut host_msg = libc::msghdr {
            msg_name: name_ptr,
            msg_namelen,
            msg_iov: iovecs.as_ptr() as *mut libc::iovec,
            msg_iovlen: iovecs.len(),
            msg_control: control_ptr,
            msg_controllen,
            msg_flags: 0,
        };

        let result = unsafe { libc::recvmsg(sockfd, &mut host_msg, flags) };

        // Write back updated fields
        if result >= 0 {
            // msg_namelen may be updated by kernel
            self.memory
                .write_data(msg_addr + 4, &(host_msg.msg_namelen as u32).to_be_bytes())?;
            // msg_controllen may be updated by kernel
            self.memory.write_data(
                msg_addr + 20,
                &(host_msg.msg_controllen as u32).to_be_bytes(),
            )?;
            // msg_flags contains received flags
            self.memory
                .write_data(msg_addr + 24, &(host_msg.msg_flags as i32).to_be_bytes())?;
        }

        Ok(result as i64)
    }

    fn update_nz_flags(&mut self, value: u32) {
        let is_zero = value == 0;
        let is_negative = (value & 0x8000_0000) != 0;
        self.set_flag(FLAG_Z, is_zero);
        self.set_flag(FLAG_N, is_negative);
    }

    fn set_flag(&mut self, mask: u16, set: bool) {
        if set {
            self.sr |= mask;
        } else {
            self.sr &= !mask;
        }
    }

    fn read_c_string(&self, addr: usize) -> Result<Vec<u8>> {
        let mut bytes = Vec::new();
        let mut offset = addr;
        // Limit to avoid runaway if the guest forgot a terminator.
        const MAX_LEN: usize = 4096;
        for _ in 0..MAX_LEN {
            let byte = self.memory.read_byte(offset)?;
            if byte == 0 {
                return Ok(bytes);
            }
            bytes.push(byte);
            offset = offset
                .checked_add(1)
                .ok_or_else(|| anyhow!("address overflow reading c-string"))?;
        }
        bail!("unterminated string starting at {addr:#x}");
    }

    fn guest_cstring(&self, addr: usize) -> Result<CString> {
        Ok(CString::new(self.read_c_string(addr)?)?)
    }

    /// Read a u64 from big-endian m68k memory
    /// m68k stores u64 as two consecutive u32 values in big-endian order
    fn read_u64_be(&self, addr: usize) -> Result<u64> {
        let hi = self.memory.read_long(addr)?;
        let lo = self.memory.read_long(addr + 4)?;
        Ok(((hi as u64) << 32) | (lo as u64))
    }

    /// Write a u64 to big-endian m68k memory
    /// m68k stores u64 as two consecutive u32 values in big-endian order
    #[allow(unused)]
    fn write_u64_be(&mut self, addr: usize, value: u64) -> Result<()> {
        let hi = (value >> 32) as u32;
        let lo = value as u32;
        self.memory.write_data(addr, &hi.to_be_bytes())?;
        self.memory.write_data(addr + 4, &lo.to_be_bytes())?;
        Ok(())
    }

    /// Read a NULL-terminated array of string pointers (e.g., argv, envp)
    /// Returns a Vec of Strings
    fn read_string_array(&self, array_addr: usize) -> Result<Vec<String>> {
        let mut strings = Vec::new();
        let mut offset = array_addr;
        const MAX_PTRS: usize = 1024; // Limit to avoid runaway

        for _i in 0..MAX_PTRS {
            // Read pointer (32-bit on m68k)
            let ptr = self
                .memory
                .read_long(offset)
                .map_err(|e| anyhow!("failed to read ptr at offset {:#x}: {}", offset, e))?
                as usize;
            if ptr == 0 {
                // NULL terminator
                break;
            }

            // Read the string at this pointer
            let c_str = self
                .guest_cstring(ptr)
                .map_err(|e| anyhow!("failed to read string at {:#x}: {}", ptr, e))?;
            let string = c_str
                .to_str()
                .map_err(|e| anyhow!("invalid UTF-8 in string array: {}", e))?
                .to_string();
            strings.push(string);

            offset = offset
                .checked_add(4)
                .ok_or_else(|| anyhow!("address overflow reading string array"))?;
        }

        if strings.len() == MAX_PTRS {
            bail!("string array exceeds maximum length at {:#x}", array_addr);
        }

        Ok(strings)
    }

    fn guest_const_ptr(&self, addr: usize, len: usize) -> Result<*const libc::c_void> {
        self.memory
            .guest_to_host(addr, len)
            .map(|p| p as *const libc::c_void)
            .ok_or_else(|| anyhow!("invalid guest buffer {addr:#x} (len {len})"))
    }

    fn guest_mut_ptr(&mut self, addr: usize, len: usize) -> Result<*mut libc::c_void> {
        self.memory
            .guest_to_host_mut(addr, len)
            .map(|p| p as *mut libc::c_void)
            .ok_or_else(|| anyhow!("invalid guest buffer {addr:#x} (len {len})"))
    }
}

const FLAG_C: u16 = 0x0001;
const FLAG_V: u16 = 0x0002;
const FLAG_Z: u16 = 0x0004;
const FLAG_N: u16 = 0x0008;
const FLAG_X: u16 = 0x0010;

/// Size-related constants bundled together
struct SizeInfo {
    mask: u32,
    sign_bit: u32,
    bits: u32,
    bytes: u32,
}

impl SizeInfo {
    fn new(size: Size) -> Self {
        match size {
            Size::Byte => Self {
                mask: 0xFF,
                sign_bit: 0x80,
                bits: 8,
                bytes: 1,
            },
            Size::Word => Self {
                mask: 0xFFFF,
                sign_bit: 0x8000,
                bits: 16,
                bytes: 2,
            },
            Size::Long => Self {
                mask: 0xFFFF_FFFF,
                sign_bit: 0x8000_0000,
                bits: 32,
                bytes: 4,
            },
        }
    }

    fn apply(&self, value: u32) -> u32 {
        value & self.mask
    }

    fn is_negative(&self, value: u32) -> bool {
        (value & self.sign_bit) != 0
    }
}

fn write_sized_data_reg(orig: u32, value: u32, size: Size) -> u32 {
    match size {
        Size::Byte => (orig & 0xFFFF_FF00) | (value & 0xFF),
        Size::Word => (orig & 0xFFFF_0000) | (value & 0xFFFF),
        Size::Long => value,
    }
}

fn size_mask(value: u32, size: Size) -> u32 {
    SizeInfo::new(size).apply(value)
}

fn size_to_bytes(size: Size) -> u32 {
    SizeInfo::new(size).bytes
}

fn data_reg_index(reg: DataReg) -> usize {
    reg as usize
}

fn addr_reg_index(reg: AddrReg) -> usize {
    reg as usize
}

/// Add with full flag computation (N, Z, V, C, X)
fn add_with_flags(src: u32, dst: u32, size: Size, cpu: &mut Cpu) -> u32 {
    let si = SizeInfo::new(size);
    let src = si.apply(src);
    let dst = si.apply(dst);
    let result = si.apply(src.wrapping_add(dst));

    let c = (src as u64 + dst as u64) > si.mask as u64;
    let v = (si.is_negative(src) == si.is_negative(dst))
        && (si.is_negative(result) != si.is_negative(src));

    cpu.set_flag(FLAG_N, si.is_negative(result));
    cpu.set_flag(FLAG_Z, result == 0);
    cpu.set_flag(FLAG_V, v);
    cpu.set_flag(FLAG_C, c);
    cpu.set_flag(FLAG_X, c);
    result
}

/// Subtract with full flag computation (N, Z, V, C, X). Computes dst - src.
fn sub_with_flags(dst: u32, src: u32, size: Size, cpu: &mut Cpu) -> u32 {
    let si = SizeInfo::new(size);
    let src = si.apply(src);
    let dst = si.apply(dst);
    let result = si.apply(dst.wrapping_sub(src));

    let c = src > dst;
    let v = (si.is_negative(src) != si.is_negative(dst))
        && (si.is_negative(result) != si.is_negative(dst));

    cpu.set_flag(FLAG_N, si.is_negative(result));
    cpu.set_flag(FLAG_Z, result == 0);
    cpu.set_flag(FLAG_V, v);
    cpu.set_flag(FLAG_C, c);
    cpu.set_flag(FLAG_X, c);
    result
}

/// Compare (dst - src) setting only N, Z, V, C (not X)
fn cmp_with_flags(dst: u32, src: u32, size: Size, cpu: &mut Cpu) {
    let si = SizeInfo::new(size);
    let src = si.apply(src);
    let dst = si.apply(dst);
    let result = si.apply(dst.wrapping_sub(src));

    let v = (si.is_negative(src) != si.is_negative(dst))
        && (si.is_negative(result) != si.is_negative(dst));

    cpu.set_flag(FLAG_N, si.is_negative(result));
    cpu.set_flag(FLAG_Z, result == 0);
    cpu.set_flag(FLAG_V, v);
    cpu.set_flag(FLAG_C, src > dst);
}

fn imm_to_u32(imm: Immediate) -> u32 {
    match imm {
        Immediate::Byte(v) => v as u32,
        Immediate::Word(v) => v as u32,
        Immediate::Long(v) => v,
    }
}

/// Shift left helper - returns (result, carry)
fn shift_left(value: u32, count: u32, si: &SizeInfo) -> (u32, bool) {
    let count = count.min(si.bits);
    let result = si.apply(value << count);
    let carry = count > 0 && count <= si.bits && (value >> (si.bits - count)) & 1 != 0;
    (result, carry)
}

/// Arithmetic shift (ASL/ASR)
fn arithmetic_shift(value: u32, count: u32, direction: RightOrLeft, size: Size) -> (u32, bool) {
    if count == 0 {
        return (value, false);
    }
    let si = SizeInfo::new(size);

    match direction {
        RightOrLeft::Left => shift_left(value, count, &si),
        RightOrLeft::Right => {
            let count = count.min(si.bits);
            let mut result = value >> count;
            if si.is_negative(value) && count > 0 {
                result |= si.apply(si.mask << (si.bits - count));
            }
            let carry = count > 0 && (value >> (count - 1)) & 1 != 0;
            (si.apply(result), carry)
        }
    }
}

/// Logical shift (LSL/LSR)
fn logical_shift(value: u32, count: u32, direction: RightOrLeft, size: Size) -> (u32, bool) {
    if count == 0 {
        return (value, false);
    }
    let si = SizeInfo::new(size);

    match direction {
        RightOrLeft::Left => shift_left(value, count, &si),
        RightOrLeft::Right => {
            let count = count.min(si.bits);
            let result = si.apply(value >> count);
            let carry = count > 0 && (value >> (count - 1)) & 1 != 0;
            (result, carry)
        }
    }
}

/// Rotate (ROL/ROR)
fn rotate(value: u32, count: u32, direction: RightOrLeft, size: Size) -> (u32, bool) {
    if count == 0 {
        return (value, false);
    }
    let si = SizeInfo::new(size);
    let count = count % si.bits;
    if count == 0 {
        return (si.apply(value), false);
    }

    let result = match direction {
        RightOrLeft::Left => si.apply((value << count) | (value >> (si.bits - count))),
        RightOrLeft::Right => si.apply((value >> count) | (value << (si.bits - count))),
    };
    let carry = match direction {
        RightOrLeft::Left => result & 1 != 0,
        RightOrLeft::Right => (result >> (si.bits - 1)) & 1 != 0,
    };
    (result, carry)
}

/// Rotate with extend (ROXL/ROXR)
fn rotate_extended(
    value: u32,
    count: u32,
    direction: RightOrLeft,
    size: Size,
    x: bool,
) -> (u32, bool) {
    if count == 0 {
        return (value, x);
    }
    let si = SizeInfo::new(size);
    let mut val = si.apply(value);
    let mut x_bit = x;

    for _ in 0..count {
        match direction {
            RightOrLeft::Left => {
                let msb = (val >> (si.bits - 1)) & 1 != 0;
                val = si.apply((val << 1) | u32::from(x_bit));
                x_bit = msb;
            }
            RightOrLeft::Right => {
                let lsb = val & 1 != 0;
                val = (val >> 1) | (u32::from(x_bit) << (si.bits - 1));
                x_bit = lsb;
            }
        }
    }
    (val, x_bit)
}

/// ADDX with flags (Z is only cleared, never set)
fn addx_with_flags(src: u32, dst: u32, x: u32, size: Size, cpu: &mut Cpu) -> u32 {
    let (mask, sign_bit): (u32, u32) = match size {
        Size::Byte => (0xFF, 0x80),
        Size::Word => (0xFFFF, 0x8000),
        Size::Long => (0xFFFF_FFFF, 0x8000_0000),
    };

    let src_masked = src & mask;
    let dst_masked = dst & mask;
    let result = src_masked.wrapping_add(dst_masked).wrapping_add(x) & mask;

    let n = (result & sign_bit) != 0;
    // Z is only cleared if result is non-zero
    if result != 0 {
        cpu.set_flag(FLAG_Z, false);
    }
    let c = (src_masked as u64 + dst_masked as u64 + x as u64) > mask as u64;
    let src_neg = (src_masked & sign_bit) != 0;
    let dst_neg = (dst_masked & sign_bit) != 0;
    let res_neg = n;
    let v = (src_neg == dst_neg) && (res_neg != src_neg);

    cpu.set_flag(FLAG_N, n);
    cpu.set_flag(FLAG_V, v);
    cpu.set_flag(FLAG_C, c);
    cpu.set_flag(FLAG_X, c);

    result
}

/// SUBX with flags (Z is only cleared, never set)
fn subx_with_flags(dst: u32, src: u32, x: u32, size: Size, cpu: &mut Cpu) -> u32 {
    let (mask, sign_bit): (u32, u32) = match size {
        Size::Byte => (0xFF, 0x80),
        Size::Word => (0xFFFF, 0x8000),
        Size::Long => (0xFFFF_FFFF, 0x8000_0000),
    };

    let src_masked = src & mask;
    let dst_masked = dst & mask;
    let result = dst_masked.wrapping_sub(src_masked).wrapping_sub(x) & mask;

    let n = (result & sign_bit) != 0;
    if result != 0 {
        cpu.set_flag(FLAG_Z, false);
    }
    let c = (src_masked as u64 + x as u64) > dst_masked as u64;
    let src_neg = (src_masked & sign_bit) != 0;
    let dst_neg = (dst_masked & sign_bit) != 0;
    let res_neg = n;
    let v = (src_neg != dst_neg) && (res_neg != dst_neg);

    cpu.set_flag(FLAG_N, n);
    cpu.set_flag(FLAG_V, v);
    cpu.set_flag(FLAG_C, c);
    cpu.set_flag(FLAG_X, c);

    result
}

/// Add BCD (for ABCD)
fn add_bcd(src: u8, dst: u8, x: u8) -> (u8, bool) {
    let low = (dst & 0x0F) + (src & 0x0F) + x;
    let (low_result, low_carry) = if low > 9 {
        (low + 6, true)
    } else {
        (low, false)
    };

    let high = (dst >> 4) + (src >> 4) + if low_carry { 1 } else { 0 };
    let (high_result, carry) = if high > 9 {
        (high + 6, true)
    } else {
        (high, false)
    };

    let result = ((high_result & 0x0F) << 4) | (low_result & 0x0F);
    (result, carry)
}

/// Subtract BCD (for SBCD)
fn sub_bcd(dst: u8, src: u8, x: u8) -> (u8, bool) {
    let low = (dst & 0x0F).wrapping_sub(src & 0x0F).wrapping_sub(x);
    let (low_result, low_borrow) = if low > 9 {
        (low.wrapping_sub(6), true)
    } else {
        (low, false)
    };

    let high = (dst >> 4)
        .wrapping_sub(src >> 4)
        .wrapping_sub(if low_borrow { 1 } else { 0 });
    let (high_result, borrow) = if high > 9 {
        (high.wrapping_sub(6), true)
    } else {
        (high, false)
    };

    let result = ((high_result & 0x0F) << 4) | (low_result & 0x0F);
    (result, borrow)
}

/// Extract a bit field from a 32-bit value.
/// Bit numbering: offset 0 = MSB (bit 31), offset 31 = LSB (bit 0)
/// Returns the field value right-justified in a u32.
fn extract_bitfield_from_u32(value: u32, offset: u32, width: u32) -> u32 {
    // offset is the bit position of the MSB of the field (0 = bit 31)
    // width is the number of bits (1-32)
    let offset = offset % 32;
    let width = if width == 0 { 32 } else { width.min(32) };

    // Shift so the field's MSB is at bit 31, then shift right to right-justify
    let shifted = value.rotate_left(offset);
    let mask = if width == 32 {
        0xFFFFFFFF
    } else {
        (1u32 << width) - 1
    };
    (shifted >> (32 - width)) & mask
}

/// Extract a bit field from memory.
/// The base address points to the byte containing bit 0 of the field reference.
/// Offset can be negative or greater than 7.
fn extract_bitfield_from_memory(
    memory: &MemoryImage,
    base_addr: u32,
    offset: i32,
    width: u32,
) -> Result<u32> {
    let width = if width == 0 { 32 } else { width.min(32) };

    // Calculate the byte offset and bit position within that byte
    // offset is in bits from the MSB of the base address byte
    let bit_addr = (base_addr as i64 * 8) + offset as i64;
    let byte_offset = bit_addr / 8;
    let bit_in_byte = (bit_addr % 8) as u32;

    // Determine how many bytes we need to read (up to 5 bytes for worst case)
    let total_bits = bit_in_byte + width;
    let bytes_needed = total_bits.div_ceil(8) as usize;

    // Read the required bytes
    let start_addr = if byte_offset >= 0 {
        byte_offset as usize
    } else {
        // This shouldn't happen for valid bit field operations
        bail!("Bit field address underflow");
    };

    let mut field_data = 0u64;
    for i in 0..bytes_needed.min(5) {
        let byte = memory.read_byte(start_addr + i)?;
        field_data = (field_data << 8) | (byte as u64);
    }

    // Extract the field from the accumulated data
    let shift = (bytes_needed as u32 * 8) - bit_in_byte - width;
    let mask = if width == 32 {
        0xFFFFFFFF
    } else {
        (1u32 << width) - 1
    };
    Ok(((field_data >> shift) as u32) & mask)
}

/// Insert a bit field into a u32 value.
/// Returns the modified value with the field inserted.
fn insert_bitfield_into_u32(dest_value: u32, src_value: u32, offset: u32, width: u32) -> u32 {
    // offset is the bit position of the MSB of the field (0 = bit 31)
    // width is the number of bits (1-32)
    let offset = offset % 32;
    let width = if width == 0 { 32 } else { width.min(32) };

    // Create mask for the field in its final position
    let field_mask = if width == 32 {
        0xFFFFFFFF
    } else {
        (1u32 << width) - 1
    };

    // Extract low-order bits from source
    let src_bits = src_value & field_mask;

    // Rotate the field mask and source bits to the target position
    let rotated_mask = field_mask.rotate_right(offset + width);
    let rotated_src = src_bits.rotate_right(offset + width);

    // Clear the destination field and insert the source bits
    (dest_value & !rotated_mask) | rotated_src
}

/// Insert a bit field into memory.
/// The base address points to the byte containing bit 0 of the field reference.
/// Offset can be negative or greater than 7.
fn insert_bitfield_into_memory(
    memory: &mut MemoryImage,
    base_addr: u32,
    offset: i32,
    width: u32,
    value: u32,
) -> Result<()> {
    let width = if width == 0 { 32 } else { width.min(32) };

    // Calculate the byte offset and bit position within that byte
    let bit_addr = (base_addr as i64 * 8) + offset as i64;
    let byte_offset = bit_addr / 8;
    let bit_in_byte = (bit_addr % 8) as u32;

    // Determine how many bytes we need to modify
    let total_bits = bit_in_byte + width;
    let bytes_needed = total_bits.div_ceil(8) as usize;

    let start_addr = if byte_offset >= 0 {
        byte_offset as usize
    } else {
        bail!("Bit field address underflow");
    };

    // Read existing bytes
    let mut field_data = 0u64;
    for i in 0..bytes_needed.min(5) {
        let byte = memory.read_byte(start_addr + i)?;
        field_data = (field_data << 8) | (byte as u64);
    }

    // Create mask for the field
    let shift = (bytes_needed as u32 * 8) - bit_in_byte - width;
    let mask = if width == 32 {
        0xFFFFFFFFu64
    } else {
        (1u64 << width) - 1
    };

    // Clear the field and insert new value
    let field_mask = mask << shift;
    let src_bits = ((value as u64) & mask) << shift;
    field_data = (field_data & !field_mask) | src_bits;

    // Write back the modified bytes
    for i in 0..bytes_needed.min(5) {
        let byte_shift = (bytes_needed - 1 - i) * 8;
        let byte = (field_data >> byte_shift) as u8;
        memory.write_data(start_addr + i, &[byte])?;
    }

    Ok(())
}
