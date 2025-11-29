use anyhow::Result;

use crate::Cpu;

impl Cpu {
    /// Helper for syscalls with pattern: syscall(path, arg2, arg3, ...)
    /// D1 = path pointer, extra_args passed directly
    pub(crate) fn sys_path1(&self, syscall_num: u32, extra_arg: i64) -> Result<i64> {
        let path_addr = self.data_regs[1] as usize;
        let path_cstr = self.guest_cstring(path_addr)?;

        Ok(unsafe { libc::syscall(syscall_num as i64, path_cstr.as_ptr(), extra_arg) })
    }
}
