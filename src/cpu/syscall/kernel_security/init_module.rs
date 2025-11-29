use anyhow::{Result, anyhow};

use crate::Cpu;

impl Cpu {
    /// init_module(module_image, len, param_values)
    pub(crate) fn sys_init_module(&mut self) -> Result<i64> {
        let module_image_ptr = self.data_regs[1] as usize;
        let len = self.data_regs[2] as usize;
        let param_values_ptr = self.data_regs[3] as usize;

        let module_image = if len > 0 {
            self.memory
                .guest_to_host(module_image_ptr, len)
                .ok_or_else(|| anyhow!("invalid module_image buffer"))?
        } else {
            std::ptr::null()
        };

        let param_values = if param_values_ptr != 0 {
            self.read_c_string(param_values_ptr)?
        } else {
            vec![0u8]
        };

        let res = unsafe { libc::syscall(175, module_image, len, param_values.as_ptr()) };
        Ok(Self::libc_to_kernel(res as i64))
    }
}
