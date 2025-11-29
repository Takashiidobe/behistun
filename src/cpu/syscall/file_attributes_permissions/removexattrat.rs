use crate::Cpu;
use anyhow::Result;

impl Cpu {
    pub(crate) fn sys_removexattrat(&mut self) -> Result<i64> {
        let (dirfd, path_ptr, name_ptr, atflags): (libc::c_int, usize, usize, libc::c_int) =
            self.get_args();

        let path = self.read_c_string(path_ptr)?;
        let name = self.read_c_string(name_ptr)?;
        let res = unsafe {
            libc::syscall(
                466, // removexattrat syscall number on x86_64 as well
                dirfd,
                path.as_ptr(),
                name.as_ptr(),
                atflags,
            )
        };
        Ok(Self::libc_to_kernel(res as i64))
    }
}
