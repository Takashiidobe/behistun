use anyhow::Result;

use crate::Cpu;

impl Cpu {
    pub(crate) fn sys_mseal(&self) -> Result<i64> {
        // Not implemented on 32-bit Linux; return -ENOSYS style code.
        Ok(-1)
    }
}
