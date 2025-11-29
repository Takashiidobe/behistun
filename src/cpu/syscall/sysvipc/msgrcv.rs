use std::convert::TryInto;

use anyhow::{Result, anyhow};

use crate::Cpu;

impl Cpu {
    pub(crate) fn sys_msgrcv(&mut self) -> Result<i64> {
        let msqid = self.data_regs[1] as i32;
        let msgp_guest = self.data_regs[2] as usize;
        let msgsz = self.data_regs[3] as usize;
        let msgtyp = self.data_regs[4] as i64;
        let msgflg = self.data_regs[5] as i32;

        let total_size = 8 + msgsz;
        let mut host_buf = vec![0u8; total_size];

        let res = unsafe { libc::syscall(70, msqid, host_buf.as_mut_ptr(), msgsz, msgtyp, msgflg) };

        if res < 0 {
            return Ok(Self::libc_to_kernel(res as i64));
        }

        let mtype_host = i64::from_ne_bytes(host_buf[0..8].try_into().unwrap());
        self.memory
            .write_data(msgp_guest, &(mtype_host as i32).to_be_bytes())?;

        if res > 0 {
            let mtext_guest = msgp_guest + 4;
            let mtext_host = self
                .memory
                .guest_to_host_mut(mtext_guest, res as usize)
                .ok_or_else(|| anyhow!("invalid message data buffer"))?;
            unsafe {
                std::ptr::copy_nonoverlapping(host_buf[8..].as_ptr(), mtext_host, res as usize);
            }
        }

        Ok(res as i64)
    }
}
