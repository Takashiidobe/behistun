use anyhow::{Result, anyhow};

use crate::Cpu;

impl Cpu {
    pub(crate) fn sys_socket_addr(&self, syscall_num: u32) -> Result<i64> {
        let (sockfd, addr_ptr, addrlen): (i32, usize, usize) = self.get_args();

        let host_addr = self
            .memory
            .guest_to_host(addr_ptr, addrlen)
            .ok_or_else(|| anyhow!("invalid sockaddr"))?;
        Ok(unsafe { libc::syscall(syscall_num as i64, sockfd, host_addr, addrlen) })
    }
}
