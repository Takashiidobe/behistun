use anyhow::Result;

use crate::Cpu;

impl Cpu {
    pub(crate) fn sys_capset(&self) -> Result<i64> {
        let hdrp_addr = self.data_regs[1] as usize;
        let datap_addr = self.data_regs[2] as usize;

        if hdrp_addr == 0 {
            return Ok(-libc::EFAULT as i64);
        }

        // Read header from guest memory (big-endian)
        let version = self.memory.read_long(hdrp_addr)?;
        let pid = self.memory.read_long(hdrp_addr + 4)? as i32;

        // Build host header
        #[repr(C)]
        struct CapUserHeader {
            version: u32,
            pid: i32,
        }

        let hdr = CapUserHeader { version, pid };

        // Determine how many data structs based on version
        let data_count = if version == 0x19980330 { 1 } else { 2 };

        // Read data from guest memory
        #[repr(C)]
        #[derive(Copy, Clone)]
        struct CapUserData {
            effective: u32,
            permitted: u32,
            inheritable: u32,
        }

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

        // Call capset (x86_64 syscall 126)
        let result = if datap_addr == 0 {
            unsafe { libc::syscall(126, &hdr as *const _, std::ptr::null::<CapUserData>()) }
        } else {
            unsafe { libc::syscall(126, &hdr as *const _, data.as_ptr()) }
        };

        Ok(Self::libc_to_kernel(result as i64))
    }
}
