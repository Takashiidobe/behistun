use anyhow::{Result, bail};

use crate::Cpu;

impl Cpu {
    /// Multiplexer syscall that dispatches to individual IPC operations
    pub(crate) fn sys_ipc(&mut self) -> Result<i64> {
        let call = self.data_regs[1];
        let first = self.data_regs[2] as i32;
        let second = self.data_regs[3] as usize;
        let third = self.data_regs[4] as usize;
        let ptr = self.data_regs[5] as usize;
        let fifth = self.data_regs[6] as i64;

        // IPC call numbers
        const SEMOP: u32 = 1;
        const SEMGET: u32 = 2;
        const SEMCTL: u32 = 3;
        const SEMTIMEDOP: u32 = 4;
        const MSGSND: u32 = 11;
        const MSGRCV: u32 = 12;
        const MSGGET: u32 = 13;
        const MSGCTL: u32 = 14;
        const SHMAT: u32 = 21;
        const SHMDT: u32 = 22;
        const SHMGET: u32 = 23;
        const SHMCTL: u32 = 24;

        match call {
            SEMGET => {
                let saved = self.data_regs;
                self.data_regs[1] = first as u32;
                self.data_regs[2] = second as u32;
                self.data_regs[3] = third as u32;
                let result = unsafe { libc::syscall(64, first, second, third) as i64 };
                self.data_regs = saved;
                Ok(Self::libc_to_kernel(result))
            }
            SEMCTL => {
                let saved = self.data_regs;
                self.data_regs[1] = first as u32;
                self.data_regs[2] = second as u32;
                self.data_regs[3] = third as u32;
                self.data_regs[4] = ptr as u32;
                let result = self.sys_semctl()?;
                self.data_regs = saved;
                Ok(result)
            }
            SHMGET => {
                let saved = self.data_regs;
                self.data_regs[1] = first as u32;
                self.data_regs[2] = second as u32;
                self.data_regs[3] = third as u32;
                let result = unsafe { libc::syscall(29, first, second, third) as i64 };
                self.data_regs = saved;
                Ok(Self::libc_to_kernel(result))
            }
            SHMCTL => {
                let saved = self.data_regs;
                self.data_regs[1] = first as u32;
                self.data_regs[2] = second as u32;
                self.data_regs[3] = ptr as u32;
                let result = self.sys_shmctl()?;
                self.data_regs = saved;
                Ok(result)
            }
            SHMAT => {
                let saved = self.data_regs;
                self.data_regs[1] = first as u32;
                self.data_regs[2] = second as u32;
                self.data_regs[3] = third as u32;
                let result = self.sys_shmat()?;
                self.data_regs = saved;
                Ok(result)
            }
            SHMDT => {
                let saved = self.data_regs;
                self.data_regs[1] = ptr as u32;
                let result = self.sys_shmdt()?;
                self.data_regs = saved;
                Ok(result)
            }
            MSGGET => {
                let saved = self.data_regs;
                self.data_regs[1] = first as u32;
                self.data_regs[2] = second as u32;
                let result = unsafe { libc::syscall(68, first, second) as i64 };
                self.data_regs = saved;
                Ok(Self::libc_to_kernel(result))
            }
            MSGSND => {
                let saved = self.data_regs;
                self.data_regs[1] = first as u32;
                self.data_regs[2] = ptr as u32;
                self.data_regs[3] = second as u32;
                self.data_regs[4] = third as u32;
                let result = self.sys_msgsnd()?;
                self.data_regs = saved;
                Ok(result)
            }
            MSGRCV => {
                let saved = self.data_regs;
                self.data_regs[1] = first as u32;
                self.data_regs[2] = ptr as u32;
                self.data_regs[3] = second as u32;
                self.data_regs[4] = fifth as u32;
                self.data_regs[5] = third as u32;
                let result = self.sys_msgrcv()?;
                self.data_regs = saved;
                Ok(result)
            }
            MSGCTL => {
                let saved = self.data_regs;
                self.data_regs[1] = first as u32;
                self.data_regs[2] = second as u32;
                self.data_regs[3] = ptr as u32;
                let result = self.sys_msgctl()?;
                self.data_regs = saved;
                Ok(result)
            }
            SEMOP | SEMTIMEDOP => bail!("semop/semtimedop not yet implemented"),
            _ => bail!("unknown ipc call number: {}", call),
        }
    }
}
