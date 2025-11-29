use anyhow::Result;

use crate::Cpu;

impl Cpu {
    /// select(nfds, readfds, writefds, exceptfds, timeout)
    pub(crate) fn sys_select(&mut self) -> Result<i64> {
        let nfds = self.data_regs[1] as i32;
        let readfds_addr = self.data_regs[2] as usize;
        let writefds_addr = self.data_regs[3] as usize;
        let exceptfds_addr = self.data_regs[4] as usize;
        let timeout_addr = self.data_regs[5] as usize;

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

        // Handle timeout (timeval structure: two longs)
        let mut timeout_opt = if timeout_addr == 0 {
            None
        } else {
            let tv_sec = self.memory.read_long(timeout_addr)? as i64;
            let tv_usec = self.memory.read_long(timeout_addr + 4)? as i64;
            Some(libc::timeval { tv_sec, tv_usec })
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
        let timeout_ptr = timeout_opt
            .as_mut()
            .map(|t| t as *mut _)
            .unwrap_or(std::ptr::null_mut());

        // Call select
        let result =
            unsafe { libc::select(nfds, readfds_ptr, writefds_ptr, exceptfds_ptr, timeout_ptr) };

        let ret = Self::libc_to_kernel(result as i64);

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
