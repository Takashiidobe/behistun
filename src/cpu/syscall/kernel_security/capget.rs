use anyhow::Result;

use crate::Cpu;

impl Cpu {
    pub(crate) fn sys_capget(&mut self) -> Result<i64> {
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

        let mut hdr = CapUserHeader { version, pid };

        // Determine how many data structs we need based on version
        // Version 1: 1 data struct, Version 2/3: 2 data structs
        let data_count = if version == 0x19980330 { 1 } else { 2 };

        // Prepare data buffer
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

        // Call capget (x86_64 syscall 125)
        let result = if datap_addr == 0 {
            // NULL datap - just checking version
            unsafe { libc::syscall(125, &mut hdr as *mut _, std::ptr::null_mut::<CapUserData>()) }
        } else {
            unsafe { libc::syscall(125, &mut hdr as *mut _, data.as_mut_ptr()) }
        };

        // Write header back (kernel may update version field)
        self.memory
            .write_data(hdrp_addr, &hdr.version.to_be_bytes())?;
        self.memory
            .write_data(hdrp_addr + 4, &hdr.pid.to_be_bytes())?;

        // Write data back if successful and datap is not NULL
        if result >= 0 && datap_addr != 0 {
            for i in 0..data_count {
                let offset = datap_addr + i * 12;
                self.memory
                    .write_data(offset, &data[i].effective.to_be_bytes())?;
                self.memory
                    .write_data(offset + 4, &data[i].permitted.to_be_bytes())?;
                self.memory
                    .write_data(offset + 8, &data[i].inheritable.to_be_bytes())?;
            }
        }

        Ok(Self::libc_to_kernel(result as i64))
    }
}
