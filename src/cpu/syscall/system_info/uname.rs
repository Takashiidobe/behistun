use anyhow::Result;

use crate::Cpu;

impl Cpu {
    pub(crate) fn sys_uname(&mut self) -> Result<i64> {
        let buf_addr = self.data_regs[1] as usize;
        let mut uts: libc::utsname = unsafe { std::mem::zeroed() };
        let result = unsafe { libc::uname(&mut uts) };
        if result == 0 {
            let field_size = 65usize;
            self.memory.write_data(
                buf_addr,
                &uts.sysname[..field_size]
                    .iter()
                    .map(|&c| c as u8)
                    .collect::<Vec<_>>(),
            )?;
            self.memory.write_data(
                buf_addr + field_size,
                &uts.nodename[..field_size]
                    .iter()
                    .map(|&c| c as u8)
                    .collect::<Vec<_>>(),
            )?;
            self.memory.write_data(
                buf_addr + field_size * 2,
                &uts.release[..field_size]
                    .iter()
                    .map(|&c| c as u8)
                    .collect::<Vec<_>>(),
            )?;
            self.memory.write_data(
                buf_addr + field_size * 3,
                &uts.version[..field_size]
                    .iter()
                    .map(|&c| c as u8)
                    .collect::<Vec<_>>(),
            )?;
            self.memory.write_data(
                buf_addr + field_size * 4,
                &uts.machine[..field_size]
                    .iter()
                    .map(|&c| c as u8)
                    .collect::<Vec<_>>(),
            )?;
        }
        Ok(result as i64)
    }
}
