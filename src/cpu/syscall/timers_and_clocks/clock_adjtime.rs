use anyhow::Result;

use crate::Cpu;

impl Cpu {
    /// clock_adjtime(clk_id, buf) / clock_adjtime64(clk_id, buf)
    pub(crate) fn sys_clock_adjtime(&mut self) -> Result<i64> {
        let _clk_id = self.data_regs[1] as libc::clockid_t;
        let tx_addr = self.data_regs[2] as usize;

        // struct timex is complex with different layouts between architectures.
        // Like adjtimex, validate the pointer and return EPERM since the guest
        // shouldn't be able to adjust the host's clock.
        if tx_addr != 0 {
            // Validate the pointer by reading first few bytes
            let _ = self.memory.read_data(tx_addr, 16)?;
        }

        // Return EPERM as would happen for unprivileged access
        Ok(-(libc::EPERM as i64))
    }
}
