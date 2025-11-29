use anyhow::Result;

use crate::Cpu;

impl Cpu {
    /// sendfile(out_fd, in_fd, offset, count)
    pub(crate) fn sys_sendfile(&mut self) -> Result<i64> {
        let out_fd = self.data_regs[1] as i32;
        let in_fd = self.data_regs[2] as i32;
        let offset_addr = self.data_regs[3] as usize;
        let count = self.data_regs[4] as usize;

        if offset_addr == 0 {
            Ok(unsafe { libc::sendfile(out_fd, in_fd, std::ptr::null_mut(), count) as i64 })
        } else {
            let mut offset = self.memory.read_long(offset_addr)? as libc::off_t;
            let result = unsafe { libc::sendfile(out_fd, in_fd, &mut offset, count) };
            if result >= 0 {
                self.memory
                    .write_data(offset_addr, &(offset as u32).to_be_bytes())?;
            }
            Ok(result as i64)
        }
    }
}
