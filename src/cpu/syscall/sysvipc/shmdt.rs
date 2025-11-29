use anyhow::{Result, anyhow};

use crate::Cpu;

impl Cpu {
    pub(crate) fn sys_shmdt(&mut self) -> Result<i64> {
        let guest_addr = self.data_regs[1] as usize;

        let segment_idx = self
            .memory
            .find_segment_index(guest_addr)
            .ok_or_else(|| anyhow!("no shared memory segment at address {:#x}", guest_addr))?;

        self.memory.remove_segment(segment_idx);

        Ok(0)
    }
}
