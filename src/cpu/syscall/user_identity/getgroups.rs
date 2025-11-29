use anyhow::Result;

use crate::Cpu;

impl Cpu {
    /// getgroups(size, list)
    pub(crate) fn sys_getgroups(&mut self) -> Result<i64> {
        let (size, list_addr): (i32, usize) = self.get_args();
        if size == 0 {
            return Ok(unsafe { libc::getgroups(0, std::ptr::null_mut()) as i64 });
        }
        let mut groups = vec![0 as libc::gid_t; size as usize];
        let result = unsafe { libc::getgroups(size, groups.as_mut_ptr()) };
        if result > 0 && list_addr != 0 {
            for (i, &gid) in groups.iter().take(result as usize).enumerate() {
                self.memory
                    .write_data(list_addr + i * 4, &(gid).to_be_bytes())?;
            }
        }
        Ok(result as i64)
    }
}
