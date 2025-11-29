use anyhow::Result;

use crate::Cpu;

impl Cpu {
    pub(crate) fn sys_getdents64(&mut self) -> Result<i64> {
        let fd = self.data_regs[1] as i32;
        let dirp = self.data_regs[2] as usize;
        let count = self.data_regs[3] as usize;

        let mut host_buf = vec![0u8; count];
        let result =
            unsafe { libc::syscall(libc::SYS_getdents64, fd, host_buf.as_mut_ptr(), count) };

        if result < 0 {
            return Ok(Self::libc_to_kernel(result));
        }

        let bytes_read = result as usize;

        if bytes_read == 0 {
            return Ok(0);
        }

        let mut host_off = 0;
        let mut guest_off = 0;

        while host_off < bytes_read {
            if host_off + 19 > bytes_read {
                break;
            }

            let d_ino = u64::from_ne_bytes(host_buf[host_off..host_off + 8].try_into()?);
            let d_off = i64::from_ne_bytes(host_buf[host_off + 8..host_off + 16].try_into()?);
            let d_reclen = u16::from_ne_bytes(host_buf[host_off + 16..host_off + 18].try_into()?);
            let d_type = host_buf[host_off + 18];

            let name_start = host_off + 19;
            let name_end = host_buf[name_start..host_off + d_reclen as usize]
                .iter()
                .position(|&b| b == 0)
                .map(|p| name_start + p)
                .unwrap_or(host_off + d_reclen as usize);

            let name_len = name_end - name_start;

            let m68k_reclen = (19 + name_len + 1).div_ceil(8) * 8;

            if guest_off + m68k_reclen > count {
                break;
            }

            self.memory
                .write_data(dirp + guest_off, &d_ino.to_be_bytes())?;
            self.memory
                .write_data(dirp + guest_off + 8, &d_off.to_be_bytes())?;
            self.memory
                .write_data(dirp + guest_off + 16, &(m68k_reclen as u16).to_be_bytes())?;
            self.memory.write_data(dirp + guest_off + 18, &[d_type])?;

            self.memory.write_data(
                dirp + guest_off + 19,
                &host_buf[name_start..name_start + name_len],
            )?;
            self.memory
                .write_data(dirp + guest_off + 19 + name_len, &[0u8])?;

            for i in (19 + name_len + 1)..m68k_reclen {
                self.memory.write_data(dirp + guest_off + i, &[0u8])?;
            }

            host_off += d_reclen as usize;
            guest_off += m68k_reclen;
        }

        Ok(guest_off as i64)
    }
}
