use anyhow::{Result, anyhow};

use crate::Cpu;

impl Cpu {
    pub(crate) fn sys_sendmsg(&mut self) -> Result<i64> {
        let (sockfd, msg_addr, flags): (i32, usize, i32) = self.get_args();

        let msg_name = self.memory.read_long(msg_addr)? as usize;
        let msg_namelen = self.memory.read_long(msg_addr + 4)?;
        let msg_iov = self.memory.read_long(msg_addr + 8)? as usize;
        let msg_iovlen = self.memory.read_long(msg_addr + 12)? as usize;
        let msg_control = self.memory.read_long(msg_addr + 16)? as usize;
        let msg_controllen = self.memory.read_long(msg_addr + 20)? as usize;

        let iovecs = if msg_iovlen > 0 {
            self.build_iovecs(msg_iov, msg_iovlen, false)?
        } else {
            Vec::new()
        };

        let name_ptr = if msg_name != 0 && msg_namelen > 0 {
            self.memory
                .guest_to_host(msg_name, msg_namelen as usize)
                .ok_or_else(|| anyhow!("invalid msg_name pointer"))?
                as *const libc::c_void
        } else {
            std::ptr::null()
        };

        let control_ptr = if msg_control != 0 && msg_controllen > 0 {
            self.memory
                .guest_to_host(msg_control, msg_controllen)
                .ok_or_else(|| anyhow!("invalid msg_control pointer"))?
                as *const libc::c_void
        } else {
            std::ptr::null()
        };

        let host_msg = libc::msghdr {
            msg_name: name_ptr as *mut libc::c_void,
            msg_namelen,
            msg_iov: iovecs.as_ptr() as *mut libc::iovec,
            msg_iovlen: iovecs.len(),
            msg_control: control_ptr as *mut libc::c_void,
            msg_controllen,
            msg_flags: 0,
        };

        let result = unsafe { libc::sendmsg(sockfd, &host_msg, flags) };
        Ok(result as i64)
    }
}
