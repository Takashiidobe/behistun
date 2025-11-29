use crate::Cpu;
use anyhow::Result;

impl Cpu {
    pub(crate) fn sys_fremovexattr(&mut self) -> Result<i64> {
        let fd = self.data_regs[1] as libc::c_int;
        let name_ptr = self.data_regs[2] as usize;

        let name = self.read_c_string(name_ptr)?;

        let res = unsafe { libc::fremovexattr(fd, name.as_ptr() as *const libc::c_char) };
        Ok(Self::libc_to_kernel(res as i64))
    }
}
