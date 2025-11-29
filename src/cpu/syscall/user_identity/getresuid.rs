use anyhow::Result;

use crate::Cpu;

impl Cpu {
    /// getresuid(ruid, euid, suid)
    pub(crate) fn sys_getresuid(&mut self) -> Result<i64> {
        let (ruid_addr, euid_addr, suid_addr) = self.get_args();
        let (mut ruid, mut euid, mut suid) = (0, 0, 0);

        let result = unsafe { libc::getresuid(&mut ruid, &mut euid, &mut suid) };
        let mapping = [(ruid_addr, ruid), (euid_addr, euid), (suid_addr, suid)];

        if result == 0 {
            for (addr, uid) in mapping {
                if addr != 0 {
                    self.memory.write_data(addr, &uid.to_be_bytes())?;
                }
            }
        }

        Ok(result as i64)
    }
}
