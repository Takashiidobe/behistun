use anyhow::Result;

use crate::Cpu;

impl Cpu {
    pub(crate) fn sys_pkey_alloc(&self) -> Result<i64> {
        let _flags = self.data_regs[1];
        let _access_rights = self.data_regs[2];

        Ok(1)
    }
}
