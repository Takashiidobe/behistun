use anyhow::Result;

use crate::Cpu;

impl Cpu {
    /// pipe(pipefd) - writes two fds to guest memory
    pub(crate) fn sys_pipe(&mut self) -> Result<i64> {
        let pipefd_addr = self.data_regs[1] as usize;
        let mut fds: [libc::c_int; 2] = [0; 2];

        let result = unsafe { libc::pipe(fds.as_mut_ptr()) };
        if result == 0 {
            // Write the two fds to guest memory (as 32-bit big-endian)
            self.memory
                .write_data(pipefd_addr, &(fds[0] as u32).to_be_bytes())?;
            self.memory
                .write_data(pipefd_addr + 4, &(fds[1] as u32).to_be_bytes())?;
        }
        Ok(result as i64)
    }

    /// pipe2(pipefd, flags) - writes two fds to guest memory
    pub(crate) fn sys_pipe2(&mut self) -> Result<i64> {
        let pipefd_addr = self.data_regs[1] as usize;
        let flags = self.data_regs[2] as libc::c_int;
        let mut fds: [libc::c_int; 2] = [0; 2];

        let result = unsafe { libc::pipe2(fds.as_mut_ptr(), flags) };
        if result == 0 {
            // Write the two fds to guest memory (as 32-bit big-endian)
            self.memory
                .write_data(pipefd_addr, &(fds[0] as u32).to_be_bytes())?;
            self.memory
                .write_data(pipefd_addr + 4, &(fds[1] as u32).to_be_bytes())?;
        }
        Ok(result as i64)
    }
}
