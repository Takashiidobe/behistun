use anyhow::{Result, anyhow};

use crate::Cpu;

impl Cpu {
    pub(crate) fn sys_getsockopt(&mut self) -> Result<i64> {
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
}
