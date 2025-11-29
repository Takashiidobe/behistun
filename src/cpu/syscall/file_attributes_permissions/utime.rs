use anyhow::Result;

use crate::Cpu;

impl Cpu {
    /// utime(path, times) - times can be NULL
    pub(crate) fn sys_utime(&self) -> Result<i64> {
        let path_addr = self.data_regs[1] as usize;
        let times_addr = self.data_regs[2] as usize;

        let path_cstr = self.guest_cstring(path_addr)?;

        if times_addr == 0 {
            // NULL times - set to current time
            Ok(unsafe { libc::utime(path_cstr.as_ptr(), std::ptr::null()) as i64 })
        } else {
            // m68k uclibc uses 64-bit time_t
            let actime_bytes: [u8; 8] = self.memory.read_data(times_addr, 8)?.try_into().unwrap();
            let actime = i64::from_be_bytes(actime_bytes) as libc::time_t;
            let modtime_bytes: [u8; 8] = self
                .memory
                .read_data(times_addr + 8, 8)?
                .try_into()
                .unwrap();
            let modtime = i64::from_be_bytes(modtime_bytes) as libc::time_t;
            let times = libc::utimbuf { actime, modtime };
            Ok(unsafe { libc::utime(path_cstr.as_ptr(), &times) as i64 })
        }
    }
}
