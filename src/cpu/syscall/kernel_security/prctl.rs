use anyhow::{anyhow, Result};

use crate::Cpu;

impl Cpu {
    pub(crate) fn sys_prctl(&mut self) -> Result<i64> {
        let option = self.data_regs[1] as i32;
        let arg2 = self.data_regs[2] as usize;
        let arg3 = self.data_regs[3] as usize;
        let arg4 = self.data_regs[4] as usize;
        let arg5 = self.data_regs[5] as usize;

        // PR_GET_PDEATHSIG (2) - arg2 is pointer to int
        if option == 2 {
            let ptr = self
                .memory
                .guest_to_host_mut(arg2, 4)
                .ok_or_else(|| anyhow!("invalid arg2 pointer for PR_GET_PDEATHSIG"))?;
            let result = unsafe { libc::prctl(option, ptr, arg3, arg4, arg5) as i64 };
            Ok(Self::libc_to_kernel(result))
        } else {
            // For other options, pass args as-is
            let result = unsafe { libc::prctl(option, arg2, arg3, arg4, arg5) as i64 };
            Ok(Self::libc_to_kernel(result))
        }
    }
}
