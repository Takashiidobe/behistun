use anyhow::{Result, anyhow};

use crate::Cpu;

impl Cpu {
    pub(crate) fn sys_setsockopt(&self) -> Result<i64> {
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
}
