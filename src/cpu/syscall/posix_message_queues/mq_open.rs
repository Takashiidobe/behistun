use anyhow::Result;

use crate::Cpu;

impl Cpu {
    pub(crate) fn sys_mq_open(&self) -> Result<i64> {
        let name_addr = self.data_regs[1] as usize;
        let oflag = self.data_regs[2] as i32;
        let mode = self.data_regs[3];
        let attr_addr = self.data_regs[4] as usize;

        let name_cstr = self.guest_cstring(name_addr)?;

        let result = if (oflag & libc::O_CREAT) != 0 {
            let attr_ptr = if attr_addr == 0 {
                std::ptr::null::<libc::mq_attr>()
            } else {
                let mq_flags = self.memory.read_long(attr_addr)? as i32 as i64;
                let mq_maxmsg = self.memory.read_long(attr_addr + 4)? as i32 as i64;
                let mq_msgsize = self.memory.read_long(attr_addr + 8)? as i32 as i64;
                let mq_curmsgs = self.memory.read_long(attr_addr + 12)? as i32 as i64;

                let mut attr: libc::mq_attr = unsafe { std::mem::zeroed() };
                attr.mq_flags = mq_flags;
                attr.mq_maxmsg = mq_maxmsg;
                attr.mq_msgsize = mq_msgsize;
                attr.mq_curmsgs = mq_curmsgs;

                Box::leak(Box::new(attr)) as *const libc::mq_attr
            };

            let res =
                unsafe { libc::syscall(240, name_cstr.as_ptr(), oflag, mode, attr_ptr) as i64 };

            if !attr_ptr.is_null() {
                unsafe {
                    let _ = Box::from_raw(attr_ptr as *mut libc::mq_attr);
                }
            }

            res
        } else {
            unsafe { libc::syscall(240, name_cstr.as_ptr(), oflag) as i64 }
        };

        Ok(Self::libc_to_kernel(result))
    }
}
