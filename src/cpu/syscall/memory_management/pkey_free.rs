use anyhow::Result;

use crate::Cpu;

impl Cpu {
    pub(crate) fn sys_pkey_free(&self) -> Result<i64> {
        let _pkey = self.data_regs[1] as i32;
        Ok(0)
    }
}
