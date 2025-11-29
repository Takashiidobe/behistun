use anyhow::Result;

use crate::Cpu;

impl Cpu {
    /// waitid(idtype, id, infop, options)
    pub(crate) fn sys_waitid(&mut self) -> Result<i64> {
        let (idtype, id, infop_addr, options): (i32, i32, usize, i32) = self.get_args();

        // Allocate host siginfo_t
        let mut infop: libc::siginfo_t = unsafe { std::mem::zeroed() };

        // Call waitid (5th parameter is rusage, always NULL for basic waitid)
        let result = unsafe {
            libc::syscall(
                libc::SYS_waitid,
                idtype,
                id,
                if infop_addr != 0 {
                    &mut infop as *mut _
                } else {
                    std::ptr::null_mut::<libc::siginfo_t>()
                },
                options,
                std::ptr::null_mut::<libc::c_void>(), // rusage (NULL)
            ) as i64
        };

        // If successful and infop is not NULL, write back siginfo_t
        if result == 0 && infop_addr != 0 {
            unsafe {
                self.memory
                    .write_data(infop_addr, &(infop.si_signo as u32).to_be_bytes())?;
                self.memory
                    .write_data(infop_addr + 4, &(infop.si_errno as u32).to_be_bytes())?;
                self.memory
                    .write_data(infop_addr + 8, &(infop.si_code as u32).to_be_bytes())?;

                let si_pid = infop.si_pid();
                let si_uid = infop.si_uid();
                let si_status = infop.si_status();

                self.memory
                    .write_data(infop_addr + 12, &(si_pid as u32).to_be_bytes())?;
                self.memory
                    .write_data(infop_addr + 16, &(si_uid as u32).to_be_bytes())?;
                self.memory
                    .write_data(infop_addr + 20, &(si_status as u32).to_be_bytes())?;
            }
        }

        Ok(Self::libc_to_kernel(result))
    }
}
