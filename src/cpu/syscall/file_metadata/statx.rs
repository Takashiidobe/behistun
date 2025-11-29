use anyhow::Result;

use crate::Cpu;

impl Cpu {
    /// statx(dirfd, pathname, flags, mask, statxbuf)
    pub(crate) fn sys_statx(&mut self) -> Result<i64> {
        let dirfd = self.data_regs[1] as i32;
        let pathname_addr = self.data_regs[2] as usize;
        let flags = self.data_regs[3] as i32;
        let mask = self.data_regs[4];
        let statxbuf_addr = self.data_regs[5] as usize;

        let pathname = self.guest_cstring(pathname_addr)?;

        // Allocate statx buffer
        let mut statxbuf: libc::statx = unsafe { std::mem::zeroed() };

        // Call statx syscall directly
        let result = unsafe {
            libc::syscall(
                libc::SYS_statx,
                dirfd,
                pathname.as_ptr(),
                flags,
                mask,
                &mut statxbuf,
            )
        };

        if result == 0 {
            // Write statx result back to guest memory (converting to m68k stat layout)
            self.write_statx(statxbuf_addr, &statxbuf)?;
        }

        Ok(result as i64)
    }
}
