use anyhow::{Result, anyhow};

use crate::Cpu;

impl Cpu {
    pub(crate) fn sys_msgsnd(&mut self) -> Result<i64> {
        let (msqid, msgp_guest, msgsz, msgflg): (i32, usize, usize, i32) = self.get_args();

        let mtype_m68k = self.memory.read_long(msgp_guest)? as i32;

        let total_size = 8 + msgsz;
        let mut host_buf = vec![0u8; total_size];

        host_buf[0..8].copy_from_slice(&(mtype_m68k as i64).to_ne_bytes());

        if msgsz > 0 {
            let mtext_guest = msgp_guest + 4;
            let mtext_data = self
                .memory
                .guest_to_host(mtext_guest, msgsz)
                .ok_or_else(|| anyhow!("invalid message data"))?;
            host_buf[8..].copy_from_slice(unsafe { std::slice::from_raw_parts(mtext_data, msgsz) });
        }

        let res = unsafe { libc::syscall(69, msqid, host_buf.as_ptr(), msgsz, msgflg) };
        Ok(Self::libc_to_kernel(res as i64))
    }
}
