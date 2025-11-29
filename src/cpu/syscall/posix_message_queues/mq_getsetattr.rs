use anyhow::Result;

use crate::Cpu;

impl Cpu {
    pub(crate) fn sys_mq_getsetattr(&mut self) -> Result<i64> {
        let mqdes = self.data_regs[1] as i32;
        let newattr_addr = self.data_regs[2] as usize;
        let oldattr_addr = self.data_regs[3] as usize;

        let newattr_ptr = if newattr_addr == 0 {
            std::ptr::null::<libc::mq_attr>()
        } else {
            let mq_flags = self.memory.read_long(newattr_addr)? as i32 as i64;
            let mq_maxmsg = self.memory.read_long(newattr_addr + 4)? as i32 as i64;
            let mq_msgsize = self.memory.read_long(newattr_addr + 8)? as i32 as i64;
            let mq_curmsgs = self.memory.read_long(newattr_addr + 12)? as i32 as i64;

            let mut newattr: libc::mq_attr = unsafe { std::mem::zeroed() };
            newattr.mq_flags = mq_flags;
            newattr.mq_maxmsg = mq_maxmsg;
            newattr.mq_msgsize = mq_msgsize;
            newattr.mq_curmsgs = mq_curmsgs;

            Box::leak(Box::new(newattr)) as *const libc::mq_attr
        };

        let mut oldattr: libc::mq_attr = unsafe { std::mem::zeroed() };
        let oldattr_ptr = if oldattr_addr == 0 {
            std::ptr::null_mut::<libc::mq_attr>()
        } else {
            &mut oldattr as *mut libc::mq_attr
        };

        let result = unsafe { libc::syscall(245, mqdes, newattr_ptr, oldattr_ptr) as i64 };

        if !newattr_ptr.is_null() {
            unsafe {
                let _ = Box::from_raw(newattr_ptr as *mut libc::mq_attr);
            }
        }

        if result >= 0 && oldattr_addr != 0 {
            self.memory
                .write_data(oldattr_addr, &(oldattr.mq_flags as i32).to_be_bytes())?;
            self.memory
                .write_data(oldattr_addr + 4, &(oldattr.mq_maxmsg as i32).to_be_bytes())?;
            self.memory
                .write_data(oldattr_addr + 8, &(oldattr.mq_msgsize as i32).to_be_bytes())?;
            self.memory.write_data(
                oldattr_addr + 12,
                &(oldattr.mq_curmsgs as i32).to_be_bytes(),
            )?;
        }

        Ok(Self::libc_to_kernel(result))
    }
}
