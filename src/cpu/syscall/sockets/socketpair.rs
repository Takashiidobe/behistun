use anyhow::Result;

use crate::Cpu;

impl Cpu {
    pub(crate) fn sys_socketpair(&mut self) -> Result<i64> {
        let domain = self.data_regs[1] as i32;
        let socktype = self.data_regs[2] as i32;
        let protocol = self.data_regs[3] as i32;
        let sv_addr = self.data_regs[4] as usize;

        let mut sv: [i32; 2] = [0; 2];
        let result = unsafe { libc::socketpair(domain, socktype, protocol, sv.as_mut_ptr()) };
        if result == 0 {
            self.memory
                .write_data(sv_addr, &(sv[0] as u32).to_be_bytes())?;
            self.memory
                .write_data(sv_addr + 4, &(sv[1] as u32).to_be_bytes())?;
        }
        Ok(result as i64)
    }
}
