use anyhow::{anyhow, Result};

use crate::Cpu;

impl Cpu {
    pub(crate) fn sys_pkey_mprotect(&self) -> Result<i64> {
        let addr = self.data_regs[1] as usize;
        let len = self.data_regs[2] as usize;
        let _prot = self.data_regs[3] as i32;
        let _pkey = self.data_regs[4] as i32;

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

        Ok(0)
    }
}
