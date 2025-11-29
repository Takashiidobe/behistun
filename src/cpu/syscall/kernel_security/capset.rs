use anyhow::Result;

use super::{CapUserData, CapUserHeader};
use crate::Cpu;

impl Cpu {
    pub(crate) fn sys_capset(&self) -> Result<i64> {
        let (hdrp_addr, datap_addr): (usize, usize) = self.get_args();

        if hdrp_addr == 0 {
            return Ok(-libc::EFAULT as i64);
        }

        // Read header from guest memory (big-endian)
        let version = self.memory.read_long(hdrp_addr)?;
        let pid = self.memory.read_long(hdrp_addr + 4)? as i32;

        let hdr = CapUserHeader { version, pid };

        // Determine how many data structs based on version
        let data_count = if version == 0x19980330 { 1 } else { 2 };

        let mut data: [CapUserData; 2] = [CapUserData {
            effective: 0,
            permitted: 0,
            inheritable: 0,
        }; 2];

        if datap_addr != 0 {
            for i in 0..data_count {
                let offset = datap_addr + i * 12;
                data[i].effective = self.memory.read_long(offset)?;
                data[i].permitted = self.memory.read_long(offset + 4)?;
                data[i].inheritable = self.memory.read_long(offset + 8)?;
            }
        }

        let result = if datap_addr == 0 {
            unsafe {
                libc::syscall(
                    libc::SYS_capset,
                    &hdr as *const _,
                    std::ptr::null::<CapUserData>(),
                )
            }
        } else {
            unsafe { libc::syscall(libc::SYS_capset, &hdr as *const _, data.as_ptr()) }
        };

        Ok(Self::libc_to_kernel(result as i64))
    }
}
