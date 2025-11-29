use anyhow::Result;

use crate::Cpu;

impl Cpu {
    /// getresuid(ruid, euid, suid)
    pub(crate) fn sys_getresuid(&mut self) -> Result<i64> {
        let ruid_addr = self.data_regs[1] as usize;
        let euid_addr = self.data_regs[2] as usize;
        let suid_addr = self.data_regs[3] as usize;

        let mut ruid: libc::uid_t = 0;
        let mut euid: libc::uid_t = 0;
        let mut suid: libc::uid_t = 0;

        let result = unsafe { libc::getresuid(&mut ruid, &mut euid, &mut suid) };
        if result == 0 {
            if ruid_addr != 0 {
                self.memory.write_data(ruid_addr, &ruid.to_be_bytes())?;
            }
            if euid_addr != 0 {
                self.memory.write_data(euid_addr, &euid.to_be_bytes())?;
            }
            if suid_addr != 0 {
                self.memory.write_data(suid_addr, &suid.to_be_bytes())?;
            }
        }
        Ok(result as i64)
    }
}
