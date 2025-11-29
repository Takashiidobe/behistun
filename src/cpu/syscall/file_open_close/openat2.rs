use anyhow::Result;

use crate::Cpu;

impl Cpu {
    /// openat2(dirfd, path, how, size)
    /// Extended version of openat with struct open_how for additional control
    pub(crate) fn sys_openat2(&self) -> Result<i64> {
        let dirfd = self.data_regs[1] as i32;
        let path_addr = self.data_regs[2] as usize;
        let how_addr = self.data_regs[3] as usize;
        let size = self.data_regs[4] as usize;

        // Read path
        let path_cstr = self.guest_cstring(path_addr)?;

        if size < 24 {
            // Size too small for valid open_how structure
            return Ok(Self::libc_to_kernel(-libc::EINVAL as i64));
        }

        // Read fields using the helper
        let m68k_flags = self.read_u64_be(how_addr)?;
        let mode = self.read_u64_be(how_addr + 8)?;
        let resolve = self.read_u64_be(how_addr + 16)?;

        // Translate flags from m68k to host
        let host_flags = Self::translate_open_flags(m68k_flags as i32) as u64;

        // Build host open_how structure
        #[repr(C)]
        struct OpenHow {
            flags: u64,
            mode: u64,
            resolve: u64,
        }

        let host_how = OpenHow {
            flags: host_flags,
            mode,
            resolve,
        };

        // Call openat2 via syscall (no libc wrapper)
        let result = unsafe {
            libc::syscall(
                437, // SYS_openat2
                dirfd,
                path_cstr.as_ptr(),
                &host_how as *const OpenHow,
                std::mem::size_of::<OpenHow>(),
            ) as i64
        };

        Ok(Self::libc_to_kernel(result))
    }
}
