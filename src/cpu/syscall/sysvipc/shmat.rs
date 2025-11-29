use anyhow::{Result, anyhow};

use crate::Cpu;
use goblin::elf::program_header;

impl Cpu {
    pub(crate) fn sys_shmat(&mut self) -> Result<i64> {
        let (shmid, shmaddr_hint, shmflg) = self.get_args();

        let host_ptr = unsafe { libc::shmat(shmid, std::ptr::null(), shmflg) as *mut u8 };

        if host_ptr == libc::MAP_FAILED as *mut u8 {
            let errno = unsafe { *libc::__errno_location() };
            return Ok(Self::libc_to_kernel(-errno as i64));
        }

        let mut shmid_ds: libc::shmid_ds = unsafe { std::mem::zeroed() };
        let stat_result = unsafe { libc::shmctl(shmid, libc::IPC_STAT, &mut shmid_ds) };
        if stat_result < 0 {
            unsafe { libc::shmdt(host_ptr as *const libc::c_void) };
            let errno = unsafe { *libc::__errno_location() };
            return Ok(Self::libc_to_kernel(-errno as i64));
        }
        let size = shmid_ds.shm_segsz;

        let guest_addr = if shmaddr_hint == 0 {
            self.memory
                .find_free_range(size)
                .ok_or_else(|| anyhow!("no free guest memory for shmat"))?
        } else {
            shmaddr_hint
        };

        let flags = if shmflg & libc::SHM_RDONLY != 0 {
            program_header::PF_R
        } else {
            program_header::PF_R | program_header::PF_W
        };

        let segment = crate::memory::MemorySegment {
            vaddr: guest_addr,
            data: crate::memory::MemoryData::Foreign {
                ptr: host_ptr,
                len: size,
                shmid,
            },
            flags,
            align: 4096,
        };

        self.memory.add_segment(segment);

        Ok(guest_addr as i64)
    }
}
