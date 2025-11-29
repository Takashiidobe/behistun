use anyhow::Result;

use crate::Cpu;

impl Cpu {
    /// setgroups(size, list)
    pub(crate) fn sys_setgroups(&self) -> Result<i64> {
        let size = self.data_regs[1] as usize;
        let list_addr = self.data_regs[2] as usize;
        let mut groups = Vec::with_capacity(size);
        for i in 0..size {
            let gid = self.memory.read_long(list_addr + i * 4)? as libc::gid_t;
            groups.push(gid);
        }
        Ok(unsafe { libc::setgroups(size, groups.as_ptr()) as i64 })
    }
}
