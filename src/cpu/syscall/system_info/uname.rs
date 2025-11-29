use anyhow::Result;

use crate::Cpu;

impl Cpu {
    pub(crate) fn sys_uname(&mut self) -> Result<i64> {
        let buf_addr = self.data_regs[1] as usize;
        let mut uts: libc::utsname = unsafe { std::mem::zeroed() };
        let result = unsafe { libc::uname(&mut uts) };
        if result == 0 {
            self.write_uname_data(&mut uts, buf_addr)?
        }
        Ok(result as i64)
    }

    fn write_uname_data(&mut self, uts: &mut libc::utsname, buf_addr: usize) -> Result<()> {
        let field_size = 65;
        self.memory.write_data(buf_addr, &write_uts(&uts.sysname))?;
        self.memory
            .write_data(buf_addr + field_size, &write_uts(&uts.nodename))?;
        self.memory.write_data(
            buf_addr + field_size * 2,
            &write_uts(&uts.release[..field_size]),
        )?;
        self.memory.write_data(
            buf_addr + field_size * 3,
            &write_uts(&uts.version[..field_size]),
        )?;
        self.memory
            .write_data(buf_addr + field_size * 4, &write_uts(&uts.machine))?;
        Ok(())
    }
}

fn write_uts(data: &[i8]) -> Vec<u8> {
    data.iter().map(|&c| c as u8).collect()
}
