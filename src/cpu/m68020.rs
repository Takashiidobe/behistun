use std::collections::BTreeMap;

use super::{M68K_TLS_TCB_SIZE, align_up};
use anyhow::{Result, anyhow, bail};

use crate::{
    decoder::{
        Abcd, Add, AddrReg, AddressingMode, Addx, And, BitFieldParam, BitOp, Condition, DataDir,
        DataReg, Decoder, DnToEa, EaToDn, EffectiveAddress, Exg, ExtMode, ImmOp, Immediate,
        Instruction, InstructionKind, Movem, Or, QuickOp, RightOrLeft, Sbcd, Shift, ShiftCount,
        Size, Sub, Subx, UnaryOp,
    },
    memory::MemoryImage,
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

pub struct Cpu {
    pub(super) data_regs: [u32; 8],
    pub(super) addr_regs: [u32; 8],
    pub(super) sr: u16,
    pub(super) pc: usize,
    pub(super) memory: MemoryImage,
    pub(super) halted: bool,
    pub(super) tls_base: u32, // Thread-local storage base address
    pub(super) tls_initialized: bool,
    pub(super) tls_memsz: usize,
    pub(super) brk: usize,
    pub(super) brk_base: usize,
    pub(super) heap_segment_base: usize,
    pub(super) stack_base: usize,
    pub(super) exe_path: String, // Path to the m68k executable being run
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
    pub(super) fn setup_initial_stack(
        &mut self,
        args: &[String],
        elf_info: &ElfInfo,
    ) -> Result<()> {
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
