use anyhow::{Result, anyhow};

use crate::Cpu;

impl Cpu {
    pub(crate) fn sys_signalfd(&self) -> Result<i64> {
        let (fd, mask_addr, flags): (i32, usize, i32) = self.get_args();

        // Mask is a sigset_t, which is typically 128 bytes on m68k
        let mask_size = std::mem::size_of::<libc::sigset_t>();
        let mask_ptr = self
            .memory
            .guest_to_host(mask_addr, mask_size)
            .ok_or_else(|| anyhow!("invalid sigset_t buffer"))?;

        let result = unsafe {
            libc::syscall(
                libc::SYS_signalfd,
                fd,
                mask_ptr as *const libc::sigset_t,
                flags,
            )
        };
        Ok(Self::libc_to_kernel(result))
    }
}
