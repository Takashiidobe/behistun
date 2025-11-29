use anyhow::{anyhow, Result};

use crate::Cpu;

impl Cpu {
    pub(crate) fn sys_getcpu(&mut self) -> Result<i64> {
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
}
