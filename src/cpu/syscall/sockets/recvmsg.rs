use anyhow::{Result, anyhow};

use crate::Cpu;

impl Cpu {
    pub(crate) fn sys_recvmsg(&mut self) -> Result<i64> {
        let (sockfd, msg_addr, flags): (i32, usize, i32) = self.get_args();

        let msg_name = self.memory.read_long(msg_addr)? as usize;
        let msg_namelen = self.memory.read_long(msg_addr + 4)?;
        let msg_iov = self.memory.read_long(msg_addr + 8)? as usize;
        let msg_iovlen = self.memory.read_long(msg_addr + 12)? as usize;
        let msg_control = self.memory.read_long(msg_addr + 16)? as usize;
        let msg_controllen = self.memory.read_long(msg_addr + 20)? as usize;

        let iovecs = if msg_iovlen > 0 {
            self.build_iovecs(msg_iov, msg_iovlen, true)?
        } else {
            Vec::new()
        };

        let name_ptr = if msg_name != 0 && msg_namelen > 0 {
            self.memory
                .guest_to_host_mut(msg_name, msg_namelen as usize)
                .ok_or_else(|| anyhow!("invalid msg_name pointer"))?
                as *mut libc::c_void
        } else {
            std::ptr::null_mut()
        };

        let control_ptr = if msg_control != 0 && msg_controllen > 0 {
            self.memory
                .guest_to_host_mut(msg_control, msg_controllen)
                .ok_or_else(|| anyhow!("invalid msg_control pointer"))?
                as *mut libc::c_void
        } else {
            std::ptr::null_mut()
        };

        let mut host_msg = libc::msghdr {
            msg_name: name_ptr,
            msg_namelen,
            msg_iov: iovecs.as_ptr() as *mut libc::iovec,
            msg_iovlen: iovecs.len(),
            msg_control: control_ptr,
            msg_controllen,
            msg_flags: 0,
        };

        let result = unsafe { libc::recvmsg(sockfd, &mut host_msg, flags) };

        if result >= 0 {
            self.memory
                .write_data(msg_addr + 4, &(host_msg.msg_namelen as u32).to_be_bytes())?;
            self.memory.write_data(
                msg_addr + 20,
                &(host_msg.msg_controllen as u32).to_be_bytes(),
            )?;
            self.memory
                .write_data(msg_addr + 24, &(host_msg.msg_flags as i32).to_be_bytes())?;
        }

        Ok(result as i64)
    }
}
