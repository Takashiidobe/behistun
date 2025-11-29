use anyhow::{Result, anyhow};

use crate::Cpu;
use crate::cpu::align_up;

impl Cpu {
    /// brk(addr) - grow/shrink the emulated heap
    pub(crate) fn sys_brk(&mut self) -> Result<i64> {
        let (requested,) = self.get_args();
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
                .ok_or_else(|| anyhow!("brk underflow"))?;
            self.memory
                .resize_segment(self.heap_segment_base, new_len)?;
        }

        // Store and return the exact requested value (like Linux)
        self.brk = target;
        Ok(self.brk as i64)
    }
}
