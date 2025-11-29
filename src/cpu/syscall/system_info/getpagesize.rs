use anyhow::Result;

use crate::Cpu;

impl Cpu {
    pub(crate) fn sys_getpagesize(&mut self) -> Result<i64> {
        let sz = unsafe { libc::sysconf(libc::_SC_PAGESIZE) };
        if sz <= 0 {
            Ok(-libc::EINVAL as i64)
        } else {
            Ok(sz as i64)
        }
    }
}
