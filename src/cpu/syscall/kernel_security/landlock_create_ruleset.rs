use anyhow::{Result, anyhow};

use crate::Cpu;

impl Cpu {
    pub(crate) fn sys_landlock_create_ruleset(&mut self) -> Result<i64> {
        let attr_addr = self.data_regs[1] as usize;
        let size = self.data_regs[2] as usize;
        let flags = self.data_regs[3];

        if attr_addr == 0 || size == 0 {
            let result = unsafe { libc::syscall(444, std::ptr::null::<u8>(), size, flags) };
            return Ok(Self::libc_to_kernel(result as i64));
        }

        let copy_len = size.min(16);
        self.memory
            .guest_to_host(attr_addr, copy_len)
            .ok_or_else(|| anyhow!("invalid landlock_ruleset_attr"))?;

        let handled_access_fs = if size >= 8 {
            self.read_u64_be(attr_addr)?
        } else {
            0
        };

        let handled_access_net = if size >= 16 {
            self.read_u64_be(attr_addr + 8)?
        } else {
            0
        };

        let mut host_attr = vec![0u8; size];
        let mut fields = [0u8; 16];
        fields[..8].copy_from_slice(&handled_access_fs.to_ne_bytes());
        if size >= 16 {
            fields[8..16].copy_from_slice(&handled_access_net.to_ne_bytes());
        }
        host_attr[..copy_len].copy_from_slice(&fields[..copy_len]);

        let result = unsafe { libc::syscall(444, host_attr.as_ptr(), size, flags) };

        Ok(Self::libc_to_kernel(result as i64))
    }
}
