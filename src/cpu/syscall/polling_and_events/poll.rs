use anyhow::Result;

use crate::Cpu;

impl Cpu {
    pub(crate) fn sys_poll(&mut self) -> Result<i64> {
        let fds_addr = self.data_regs[1] as usize;
        let nfds = self.data_regs[2] as usize;
        let timeout = self.data_regs[3] as i32;

        let mut pollfds = Vec::with_capacity(nfds);
        for i in 0..nfds {
            let fd = self.memory.read_long(fds_addr + i * 8)? as i32;
            let events = self.memory.read_word(fds_addr + i * 8 + 4)? as i16;
            pollfds.push(libc::pollfd {
                fd,
                events,
                revents: 0,
            });
        }

        let result = unsafe { libc::poll(pollfds.as_mut_ptr(), nfds as libc::nfds_t, timeout) };

        for (i, pfd) in pollfds.iter().enumerate() {
            self.memory
                .write_data(fds_addr + i * 8 + 6, &(pfd.revents as u16).to_be_bytes())?;
        }
        Ok(result as i64)
    }
}
