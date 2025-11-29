use anyhow::{Result, anyhow};

use crate::Cpu;

impl Cpu {
    /// semctl(semid, semnum, cmd, arg)
    /// arg is a union semun which can be:
    ///   - int val (for SETVAL)
    ///   - struct semid_ds *buf (for IPC_STAT, IPC_SET)
    ///   - unsigned short *array (for GETALL, SETALL)
    pub(crate) fn sys_semctl(&mut self) -> Result<i64> {
        let (semid, semnum, cmd, arg_val): (i32, i32, i32, usize) = self.get_args();

        // Commands that don't need the 4th argument
        if cmd == libc::IPC_RMID {
            let res = unsafe { libc::syscall(66, semid, semnum, cmd, 0) };
            return Ok(Self::libc_to_kernel(res as i64));
        }

        // SETVAL uses arg.val (passed as integer)
        const SETVAL: i32 = 16;
        if cmd == SETVAL {
            let res = unsafe { libc::syscall(66, semid, semnum, cmd, arg_val as i32) };
            return Ok(Self::libc_to_kernel(res as i64));
        }

        // GETVAL, GETPID, GETNCNT, GETZCNT don't use arg
        const GETVAL: i32 = 12;
        const GETPID: i32 = 11;
        const GETNCNT: i32 = 14;
        const GETZCNT: i32 = 15;
        if cmd == GETVAL || cmd == GETPID || cmd == GETNCNT || cmd == GETZCNT {
            let res = unsafe { libc::syscall(66, semid, semnum, cmd, 0) };
            return Ok(Self::libc_to_kernel(res as i64));
        }

        // IPC_STAT, IPC_SET use arg.buf (struct semid_ds*)
        if cmd == libc::IPC_STAT || cmd == libc::IPC_SET {
            if arg_val == 0 {
                let res = unsafe {
                    libc::syscall(66, semid, semnum, cmd, std::ptr::null_mut::<libc::c_void>())
                };
                return Ok(Self::libc_to_kernel(res as i64));
            }
            let buf_host = self
                .memory
                .guest_to_host_mut(arg_val, 128)
                .ok_or_else(|| anyhow!("invalid semid_ds buffer"))?;
            let res = unsafe { libc::syscall(66, semid, semnum, cmd, buf_host) };
            return Ok(Self::libc_to_kernel(res as i64));
        }

        // GETALL, SETALL use arg.array (unsigned short*)
        const GETALL: i32 = 13;
        const SETALL: i32 = 17;
        if cmd == GETALL || cmd == SETALL {
            if arg_val == 0 {
                return Ok(Self::libc_to_kernel(-libc::EINVAL as i64));
            }
            let array_host = self
                .memory
                .guest_to_host_mut(arg_val, 512) // 256 shorts * 2 bytes
                .ok_or_else(|| anyhow!("invalid semaphore array"))?;
            let res = unsafe { libc::syscall(66, semid, semnum, cmd, array_host) };
            return Ok(Self::libc_to_kernel(res as i64));
        }

        // Default: pass through arg as-is (for other commands like IPC_INFO)
        let res = unsafe { libc::syscall(66, semid, semnum, cmd, arg_val) };
        Ok(Self::libc_to_kernel(res as i64))
    }
}
