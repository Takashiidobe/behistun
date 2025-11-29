use anyhow::{Result, anyhow};

use crate::Cpu;

impl Cpu {
    pub(crate) fn sys_landlock_add_rule(&mut self) -> Result<i64> {
        let ruleset_fd = self.data_regs[1] as i32;
        let rule_type = self.data_regs[2];
        let rule_attr_addr = self.data_regs[3] as usize;
        let flags = self.data_regs[4];

        const LANDLOCK_RULE_PATH_BENEATH: u32 = 1;
        const LANDLOCK_RULE_NET_PORT: u32 = 2;

        if rule_attr_addr == 0 {
            let result =
                unsafe { libc::syscall(445, ruleset_fd, rule_type, std::ptr::null::<u8>(), flags) };
            return Ok(Self::libc_to_kernel(result as i64));
        }

        let result = match rule_type {
            LANDLOCK_RULE_PATH_BENEATH => {
                self.memory
                    .guest_to_host(rule_attr_addr, 12)
                    .ok_or_else(|| anyhow!("invalid landlock_path_beneath_attr"))?;

                let allowed_access = self.read_u64_be(rule_attr_addr)?;
                let parent_fd = self.memory.read_long(rule_attr_addr + 8)? as i32;

                let mut host_attr = [0u8; 16];
                host_attr[..8].copy_from_slice(&allowed_access.to_ne_bytes());
                host_attr[8..12].copy_from_slice(&parent_fd.to_ne_bytes());

                unsafe { libc::syscall(445, ruleset_fd, rule_type, host_attr.as_ptr(), flags) }
            }
            LANDLOCK_RULE_NET_PORT => {
                self.memory
                    .guest_to_host(rule_attr_addr, 16)
                    .ok_or_else(|| anyhow!("invalid landlock_net_port_attr"))?;

                let allowed_access = self.read_u64_be(rule_attr_addr)?;
                let port = self.read_u64_be(rule_attr_addr + 8)?;

                let mut host_attr = [0u8; 16];
                host_attr[..8].copy_from_slice(&allowed_access.to_ne_bytes());
                host_attr[8..16].copy_from_slice(&port.to_ne_bytes());

                unsafe { libc::syscall(445, ruleset_fd, rule_type, host_attr.as_ptr(), flags) }
            }
            _ => return Ok(Self::libc_to_kernel(-libc::EINVAL as i64)),
        };

        Ok(Self::libc_to_kernel(result as i64))
    }
}
