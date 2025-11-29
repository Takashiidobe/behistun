use anyhow::{Result, anyhow};

use crate::Cpu;

impl Cpu {
    pub(crate) fn sys_getsockname(&mut self) -> Result<i64> {
        let sockfd = self.data_regs[1] as i32;
        let addr_ptr = self.data_regs[2] as usize;
        let addrlen_ptr = self.data_regs[3] as usize;

        let mut addrlen = self.memory.read_long(addrlen_ptr)? as libc::socklen_t;
        let host_addr = self
            .memory
            .guest_to_host_mut(addr_ptr, addrlen as usize)
            .ok_or_else(|| anyhow!("invalid sockaddr buffer"))?;

        let result =
            unsafe { libc::getsockname(sockfd, host_addr as *mut libc::sockaddr, &mut addrlen) };
        if result == 0 {
            self.memory
                .write_data(addrlen_ptr, &(addrlen as u32).to_be_bytes())?;
        }
        Ok(result as i64)
    }
}
