use anyhow::{Result, anyhow};

use crate::Cpu;

impl Cpu {
    pub(crate) fn sys_recvfrom(&mut self) -> Result<i64> {
        let (sockfd, buf_ptr, len, flags, src_addr): (i32, usize, usize, i32, usize) =
            self.get_args();

        let host_buf = self
            .memory
            .guest_to_host_mut(buf_ptr, len)
            .ok_or_else(|| anyhow!("invalid recvfrom buffer"))?;
        let (host_addr, addrlen) = if src_addr != 0 {
            let addrlen: libc::socklen_t = 128;
            let host_addr = self
                .memory
                .guest_to_host_mut(src_addr, addrlen as usize)
                .ok_or_else(|| anyhow!("invalid src_addr buffer"))?;
            (addrlen, host_addr)
        } else {
            (0, std::ptr::null_mut())
        };

        Ok(unsafe {
            libc::recvfrom(
                sockfd,
                host_buf as *mut libc::c_void,
                len,
                flags,
                host_addr as *mut libc::sockaddr,
                addrlen as *mut u32,
            ) as i64
        })
    }
}
