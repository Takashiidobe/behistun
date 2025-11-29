use anyhow::Result;

use crate::Cpu;

impl Cpu {
    /// readlink(path, buf, size)
    pub(crate) fn sys_readlink(&mut self) -> Result<i64> {
        let path_addr = self.data_regs[1] as usize;
        let buf_addr = self.data_regs[2] as usize;
        let size = self.data_regs[3] as usize;
        let path = self.guest_cstring(path_addr)?;
        let host_buf = self.guest_mut_ptr(buf_addr, size)?;

        // Handle /proc/self/exe specially - return the m68k binary path
        let path_str = path.to_str().unwrap_or("");
        if path_str == "/proc/self/exe" {
            let exe_bytes = self.exe_path.as_bytes();
            let copy_len = exe_bytes.len().min(size);
            unsafe {
                std::ptr::copy_nonoverlapping(exe_bytes.as_ptr(), host_buf as *mut u8, copy_len);
            }
            return Ok(copy_len as i64);
        }

        Ok(unsafe { libc::readlink(path.as_ptr(), host_buf as *mut i8, size) as i64 })
    }
}
