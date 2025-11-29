use crate::Cpu;
use anyhow::Result;

impl Cpu {
    pub(crate) fn sys_removexattr(&mut self) -> Result<i64> {
        let path_ptr = self.data_regs[1] as usize;
        let name_ptr = self.data_regs[2] as usize;

        let path = self.read_c_string(path_ptr)?;
        let name = self.read_c_string(name_ptr)?;

        let res = unsafe {
            libc::removexattr(
                path.as_ptr() as *const libc::c_char,
                name.as_ptr() as *const libc::c_char,
            )
        };
        Ok(Self::libc_to_kernel(res as i64))
    }
}
