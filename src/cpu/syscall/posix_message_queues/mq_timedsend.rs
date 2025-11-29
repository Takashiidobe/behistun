use anyhow::{Result, anyhow};

use crate::Cpu;
use std::convert::TryInto;

impl Cpu {
    pub(crate) fn sys_mq_timedsend(&self) -> Result<i64> {
        let mqdes = self.data_regs[1] as i32;
        let msg_ptr_guest = self.data_regs[2] as usize;
        let msg_len = self.data_regs[3] as usize;
        let msg_prio = self.data_regs[4];
        let timeout_addr = self.data_regs[5] as usize;

        let msg_ptr_host = if msg_len > 0 {
            self.memory
                .guest_to_host(msg_ptr_guest, msg_len)
                .ok_or_else(|| anyhow!("mq_timedsend: invalid message buffer"))?
        } else {
            std::ptr::null()
        };

        let timeout_ptr = if timeout_addr == 0 {
            std::ptr::null::<libc::timespec>()
        } else {
            let tv_sec_bytes: [u8; 8] = self.memory.read_data(timeout_addr, 8)?.try_into().unwrap();
            let tv_sec = i64::from_be_bytes(tv_sec_bytes);
            let tv_nsec = self.memory.read_long(timeout_addr + 8)? as i64;

            let timeout = libc::timespec { tv_sec, tv_nsec };
            Box::leak(Box::new(timeout)) as *const libc::timespec
        };

        let result =
            unsafe { libc::syscall(242, mqdes, msg_ptr_host, msg_len, msg_prio, timeout_ptr) };

        if !timeout_ptr.is_null() {
            unsafe {
                let _ = Box::from_raw(timeout_ptr as *mut libc::timespec);
            }
        }

        Ok(Self::libc_to_kernel(result as i64))
    }
}
