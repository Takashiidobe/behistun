use anyhow::Result;

use crate::Cpu;

impl Cpu {
    /// pselect6_time64(nfds, readfds, writefds, exceptfds, timeout, sigmask)
    pub(crate) fn sys_pselect6(&mut self) -> Result<i64> {
        let nfds = self.data_regs[1] as i32;
        let readfds_addr = self.data_regs[2] as usize;
        let writefds_addr = self.data_regs[3] as usize;
        let exceptfds_addr = self.data_regs[4] as usize;
        let timeout_addr = self.data_regs[5] as usize;
        // Note: sigmask (6th arg) would be on stack for m68k, but we'll ignore it for now

        // Convert guest fd_sets to host format
        let mut readfds_opt = if readfds_addr != 0 {
            Some(self.guest_to_host_fdset(readfds_addr, nfds)?)
        } else {
            None
        };

        let mut writefds_opt = if writefds_addr != 0 {
            Some(self.guest_to_host_fdset(writefds_addr, nfds)?)
        } else {
            None
        };

        let mut exceptfds_opt = if exceptfds_addr != 0 {
            Some(self.guest_to_host_fdset(exceptfds_addr, nfds)?)
        } else {
            None
        };

        // Handle timeout
        let timeout_opt = if timeout_addr == 0 {
            None
        } else {
            // Read the time64 timespec structure from guest memory (two 64-bit fields)
            // Each 64-bit field is stored as two 32-bit values (big-endian on m68k)
            let tv_sec_hi = self.memory.read_long(timeout_addr)? as i64;
            let tv_sec_lo = self.memory.read_long(timeout_addr + 4)? as i64;
            let tv_sec = (tv_sec_hi << 32) | (tv_sec_lo & 0xFFFFFFFF);

            let tv_nsec_hi = self.memory.read_long(timeout_addr + 8)? as i64;
            let tv_nsec_lo = self.memory.read_long(timeout_addr + 12)? as i64;
            let tv_nsec = (tv_nsec_hi << 32) | (tv_nsec_lo & 0xFFFFFFFF);

            Some(libc::timespec { tv_sec, tv_nsec })
        };

        // Get pointers to fd_sets
        let readfds_ptr = readfds_opt
            .as_mut()
            .map(|s| s as *mut _)
            .unwrap_or(std::ptr::null_mut());
        let writefds_ptr = writefds_opt
            .as_mut()
            .map(|s| s as *mut _)
            .unwrap_or(std::ptr::null_mut());
        let exceptfds_ptr = exceptfds_opt
            .as_mut()
            .map(|s| s as *mut _)
            .unwrap_or(std::ptr::null_mut());

        // Call pselect6 (x86_64 syscall 270)
        let result = unsafe {
            if let Some(ref timeout) = timeout_opt {
                libc::syscall(
                    libc::SYS_pselect6,
                    nfds,
                    readfds_ptr,
                    writefds_ptr,
                    exceptfds_ptr,
                    timeout as *const libc::timespec,
                    std::ptr::null::<libc::sigset_t>(), // sigmask
                )
            } else {
                libc::syscall(
                    libc::SYS_pselect6,
                    nfds,
                    readfds_ptr,
                    writefds_ptr,
                    exceptfds_ptr,
                    std::ptr::null::<libc::timespec>(),
                    std::ptr::null::<libc::sigset_t>(), // sigmask
                )
            }
        };

        let ret = Self::libc_to_kernel(result);

        // Copy modified fd_sets back to guest memory
        if ret >= 0 {
            if let Some(ref readfds) = readfds_opt
                && readfds_addr != 0
            {
                self.host_to_guest_fdset(readfds, readfds_addr, nfds)?;
            }
            if let Some(ref writefds) = writefds_opt
                && writefds_addr != 0
            {
                self.host_to_guest_fdset(writefds, writefds_addr, nfds)?;
            }
            if let Some(ref exceptfds) = exceptfds_opt
                && exceptfds_addr != 0
            {
                self.host_to_guest_fdset(exceptfds, exceptfds_addr, nfds)?;
            }
        }

        Ok(ret)
    }
}
