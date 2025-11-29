use anyhow::Result;

use crate::Cpu;

impl Cpu {
    /// getresgid(rgid, egid, sgid)
    pub(crate) fn sys_getresgid(&mut self) -> Result<i64> {
        let rgid_addr = self.data_regs[1] as usize;
        let egid_addr = self.data_regs[2] as usize;
        let sgid_addr = self.data_regs[3] as usize;

        let mut rgid: libc::gid_t = 0;
        let mut egid: libc::gid_t = 0;
        let mut sgid: libc::gid_t = 0;

        let result = unsafe { libc::getresgid(&mut rgid, &mut egid, &mut sgid) };
        if result == 0 {
            if rgid_addr != 0 {
                self.memory.write_data(rgid_addr, &rgid.to_be_bytes())?;
            }
            if egid_addr != 0 {
                self.memory.write_data(egid_addr, &egid.to_be_bytes())?;
            }
            if sgid_addr != 0 {
                self.memory.write_data(sgid_addr, &sgid.to_be_bytes())?;
            }
        }
        Ok(result as i64)
    }
}
