use anyhow::{Result, anyhow};

use crate::Cpu;

impl Cpu {
    /// mincore(addr, length, vec)
    pub(crate) fn sys_mincore(&mut self) -> Result<i64> {
        let addr = self.data_regs[1] as usize;
        let length = self.data_regs[2] as usize;
        let vec_ptr = self.data_regs[3] as usize;

        const PAGE_SIZE: usize = 4096;
        let num_pages = length.div_ceil(PAGE_SIZE);

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
}
