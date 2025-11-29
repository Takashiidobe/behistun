use anyhow::Result;

use crate::Cpu;

impl Cpu {
    /// statfs(path, buf)
    pub(crate) fn sys_statfs(&mut self) -> Result<i64> {
        let path_addr = self.data_regs[1] as usize;
        let buf_addr = self.data_regs[2] as usize;
        let path = self.guest_cstring(path_addr)?;
        let mut statfs: libc::statfs = unsafe { std::mem::zeroed() };
        let result = unsafe { libc::statfs(path.as_ptr(), &mut statfs) };
        if result == 0 {
            self.write_statfs(buf_addr, &statfs)?;
        }
        Ok(result as i64)
    }

    /// fstatfs(fd, buf)
    pub(crate) fn sys_fstatfs(&mut self) -> Result<i64> {
        let fd = self.data_regs[1] as i32;
        let buf_addr = self.data_regs[2] as usize;
        let mut statfs: libc::statfs = unsafe { std::mem::zeroed() };
        let result = unsafe { libc::fstatfs(fd, &mut statfs) };
        if result == 0 {
            self.write_statfs(buf_addr, &statfs)?;
        }
        Ok(result as i64)
    }

    fn write_statfs(&mut self, addr: usize, s: &libc::statfs) -> Result<()> {
        // m68k statfs struct (simplified - key fields)
        self.memory
            .write_data(addr, &(s.f_type as u32).to_be_bytes())?;
        self.memory
            .write_data(addr + 4, &(s.f_bsize as u32).to_be_bytes())?;
        self.memory
            .write_data(addr + 8, &(s.f_blocks as u32).to_be_bytes())?;
        self.memory
            .write_data(addr + 12, &(s.f_bfree as u32).to_be_bytes())?;
        self.memory
            .write_data(addr + 16, &(s.f_bavail as u32).to_be_bytes())?;
        self.memory
            .write_data(addr + 20, &(s.f_files as u32).to_be_bytes())?;
        self.memory
            .write_data(addr + 24, &(s.f_ffree as u32).to_be_bytes())?;
        Ok(())
    }
}
