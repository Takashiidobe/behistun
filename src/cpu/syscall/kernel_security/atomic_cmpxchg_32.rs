use anyhow::Result;

use crate::Cpu;

impl Cpu {
    /// atomic_cmpxchg_32(uaddr, oldval, newval) - m68k only
    /// Returns the previous value at *uaddr.
    pub(crate) fn sys_atomic_cmpxchg_32(&mut self) -> Result<i64> {
        let addr = self.data_regs[1] as usize;
        let old = self.data_regs[2];
        let new = self.data_regs[3];

        let current = self.memory.read_long(addr)?;

        if current == old {
            self.memory.write_data(addr, &new.to_be_bytes())?;
        }
        Ok(current as i64)
    }
}
