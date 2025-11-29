use anyhow::Result;

use crate::Cpu;

impl Cpu {
    /// getresgid(rgid, egid, sgid)
    pub(crate) fn sys_getresgid(&mut self) -> Result<i64> {
        let (rgid_addr, egid_addr, sgid_addr) = self.get_args();
        let (mut rgid, mut egid, mut sgid) = (0, 0, 0);

        let result = unsafe { libc::getresgid(&mut rgid, &mut egid, &mut sgid) };
        let mapping = [(rgid_addr, rgid), (egid_addr, egid), (sgid_addr, sgid)];

        if result == 0 {
            for (addr, gid) in mapping {
                if addr != 0 {
                    self.memory.write_data(addr, &gid.to_be_bytes())?;
                }
            }
        }
        Ok(result as i64)
    }
}
