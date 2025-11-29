use anyhow::Result;

use crate::Cpu;

impl Cpu {
    /// times(buf) - writes struct tms to guest memory
    pub(crate) fn sys_times(&mut self) -> Result<i64> {
        let buf_addr = self.data_regs[1] as usize;
        let mut tms = libc::tms {
            tms_utime: 0,
            tms_stime: 0,
            tms_cutime: 0,
            tms_cstime: 0,
        };

        let result = unsafe { libc::times(&mut tms) };
        if result != -1 && buf_addr != 0 {
            // Write struct tms to guest memory (four 32-bit clock_t values on m68k)
            self.memory
                .write_data(buf_addr, &(tms.tms_utime as u32).to_be_bytes())?;
            self.memory
                .write_data(buf_addr + 4, &(tms.tms_stime as u32).to_be_bytes())?;
            self.memory
                .write_data(buf_addr + 8, &(tms.tms_cutime as u32).to_be_bytes())?;
            self.memory
                .write_data(buf_addr + 12, &(tms.tms_cstime as u32).to_be_bytes())?;
        }
        Ok(result)
    }
}
