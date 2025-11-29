use anyhow::{Result, anyhow};

use crate::Cpu;

impl Cpu {
    pub(crate) fn sys_msgctl(&mut self) -> Result<i64> {
        let (msqid, cmd, buf_ptr): (i32, i32, usize) = self.get_args();

        if cmd == libc::IPC_RMID || buf_ptr == 0 {
            let res =
                unsafe { libc::syscall(71, msqid, cmd, std::ptr::null_mut::<libc::c_void>()) };
            return Ok(Self::libc_to_kernel(res as i64));
        }

        let buf_host = self
            .memory
            .guest_to_host_mut(buf_ptr, 128)
            .ok_or_else(|| anyhow!("invalid msqid_ds buffer"))?;

        let res = unsafe { libc::syscall(71, msqid, cmd, buf_host) };
        Ok(Self::libc_to_kernel(res as i64))
    }
}
