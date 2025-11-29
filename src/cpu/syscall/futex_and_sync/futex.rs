use anyhow::{Result, anyhow};

use crate::Cpu;

impl Cpu {
    /// futex(uaddr, op, val, timeout, uaddr2, val3)
    /// Fast userspace mutex - translates guest pointers to host
    pub(crate) fn sys_futex(&mut self) -> Result<i64> {
        let (uaddr_guest, op, val, timeout_guest, uaddr2_guest): (usize, i32, i32, usize, usize) =
            self.get_args();

        // Get 6th argument from stack (m68k ABI passes 6th arg on stack)
        let sp = self.addr_regs[7] as usize;
        let val3 = if sp != 0 {
            self.memory.read_long(sp).unwrap_or(0) as i32
        } else {
            0
        };

        // Translate main futex address to host pointer
        let uaddr_host = self
            .memory
            .guest_to_host_mut(uaddr_guest, 4)
            .ok_or_else(|| anyhow!("invalid futex address {:#x}", uaddr_guest))?
            as *mut i32;

        // Handle timeout parameter if present
        // Operations like FUTEX_WAIT use timeout, others ignore it
        let timeout_opt = if timeout_guest != 0 {
            // m68k uclibc uses 64-bit time_t
            let tv_sec_bytes: [u8; 8] =
                self.memory.read_data(timeout_guest, 8)?.try_into().unwrap();
            let tv_sec = i64::from_be_bytes(tv_sec_bytes);
            let tv_nsec = self.memory.read_long(timeout_guest + 8)? as i64;
            Some(libc::timespec { tv_sec, tv_nsec })
        } else {
            None
        };

        // Handle uaddr2 for REQUEUE and CMP_REQUEUE operations
        let uaddr2_host = if uaddr2_guest != 0 {
            self.memory
                .guest_to_host_mut(uaddr2_guest, 4)
                .ok_or_else(|| anyhow!("invalid futex uaddr2 {:#x}", uaddr2_guest))?
                as *mut i32
        } else {
            std::ptr::null_mut()
        };

        // Call host futex syscall with translated pointers
        let result = unsafe {
            libc::syscall(
                libc::SYS_futex,
                uaddr_host,
                op,
                val,
                timeout_opt
                    .as_ref()
                    .map(|t| t as *const _)
                    .unwrap_or(std::ptr::null()),
                uaddr2_host,
                val3,
            )
        };

        Ok(Self::libc_to_kernel(result))
    }
}
