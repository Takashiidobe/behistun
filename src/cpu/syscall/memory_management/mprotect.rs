use anyhow::{Result, anyhow};

use crate::Cpu;

impl Cpu {
    /// mprotect(addr, len, prot)
    pub(crate) fn sys_mprotect(&self) -> Result<i64> {
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
}
