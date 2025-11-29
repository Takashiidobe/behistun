use anyhow::{Result, anyhow, bail};

use crate::Cpu;

impl Cpu {
    /// clone(flags, stack, parent_tid, child_tid, tls)
    /// On x86_64: clone(flags, stack, parent_tid, child_tid, tls)
    /// On m68k: clone(flags, stack, parent_tid, child_tid, tls) - same order
    pub(crate) fn sys_clone(&mut self) -> Result<i64> {
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
}
