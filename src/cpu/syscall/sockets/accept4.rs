use anyhow::{Result, anyhow};

use crate::Cpu;

impl Cpu {
    pub(crate) fn sys_accept4(&mut self) -> Result<i64> {
        let (sockfd, addr_ptr, addrlen_ptr, flags): (i32, usize, usize, i32) = self.get_args();

        if addr_ptr == 0 {
            return Ok(unsafe {
                libc::syscall(
                    libc::SYS_accept4,
                    sockfd,
                    std::ptr::null::<u8>(),
                    std::ptr::null::<u32>(),
                    flags,
                )
            });
        }

        let mut addrlen = self.memory.read_long(addrlen_ptr)?;
        let host_addr = self
            .memory
            .guest_to_host_mut(addr_ptr, addrlen as usize)
            .ok_or_else(|| anyhow!("invalid sockaddr buffer"))?;

        let result = unsafe {
            libc::syscall(
                libc::SYS_accept4,
                sockfd,
                host_addr,
                &mut addrlen as *mut u32,
                flags,
            )
        };
        if result >= 0 {
            self.memory
                .write_data(addrlen_ptr, &addrlen.to_be_bytes())?;
        }
        Ok(result)
    }
}
