use crate::Cpu;
use anyhow::Result;

impl Cpu {
    pub(crate) fn sys_removexattr(&mut self) -> Result<i64> {
        let (path_ptr, name_ptr): (usize, usize) = self.get_args();

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
