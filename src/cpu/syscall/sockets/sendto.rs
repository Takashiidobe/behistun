use anyhow::{Result, anyhow};

use crate::Cpu;

impl Cpu {
    pub(crate) fn sys_sendto(&self) -> Result<i64> {
        let dest_addr = self.data_regs[5] as usize;
        let (sockfd, buf_ptr, len, flags, _, addrlen): (
            i32,
            usize,
            usize,
            i32,
            usize,
            libc::socklen_t,
        ) = self.get_args();

        let host_buf = self
            .memory
            .guest_to_host(buf_ptr, len)
            .ok_or_else(|| anyhow!("invalid sendto buffer"))?;

        let host_addr = if dest_addr != 0 {
            self.memory
                .guest_to_host(dest_addr, addrlen as usize)
                .ok_or_else(|| anyhow!("error translating sockaddr"))?
        } else {
            std::ptr::null()
        };

        Ok(unsafe {
            libc::sendto(
                sockfd,
                host_buf as *const libc::c_void,
                len,
                flags,
                host_addr as *const libc::sockaddr,
                0,
            ) as i64
        })
    }
}
