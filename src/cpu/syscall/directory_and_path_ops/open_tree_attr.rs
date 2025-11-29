use anyhow::Result;

use crate::Cpu;

impl Cpu {
    /// open_tree_attr(dirfd, path, flags, attr, size)
    /// Extended open_tree with mount attribute modification
    pub(crate) fn sys_open_tree_attr(&self) -> Result<i64> {
        let dirfd = self.data_regs[1] as i32;
        let path_addr = self.data_regs[2] as usize;
        let flags = self.data_regs[3];
        let attr_addr = self.data_regs[4] as usize;
        let size = self.data_regs[5] as usize;

        // Read path
        let path_cstr = self.guest_cstring(path_addr)?;

        // Handle NULL attr or zero size - behaves like open_tree()
        if attr_addr == 0 || size == 0 {
            let result = unsafe {
                libc::syscall(
                    467, // SYS_open_tree_attr
                    dirfd,
                    path_cstr.as_ptr(),
                    flags,
                    std::ptr::null::<libc::c_void>(),
                    0,
                ) as i64
            };
            return Ok(Self::libc_to_kernel(result));
        }

        // Read struct mount_attr from guest memory (m68k big-endian)
        // struct mount_attr {
        //     u64 attr_set;     // 0: 8 bytes
        //     u64 attr_clr;     // 8: 8 bytes
        //     u64 propagation;  // 16: 8 bytes
        //     u64 userns_fd;    // 24: 8 bytes
        // }
        // Total: 32 bytes minimum

        if size < 32 {
            // Size too small for valid mount_attr structure
            return Ok(Self::libc_to_kernel(-libc::EINVAL as i64));
        }

        // Read fields using the new helper
        let attr_set = self.read_u64_be(attr_addr)?;
        let attr_clr = self.read_u64_be(attr_addr + 8)?;
        let propagation = self.read_u64_be(attr_addr + 16)?;
        let userns_fd = self.read_u64_be(attr_addr + 24)?;

        // Build host mount_attr structure
        #[repr(C)]
        struct MountAttr {
            attr_set: u64,
            attr_clr: u64,
            propagation: u64,
            userns_fd: u64,
        }

        let host_attr = MountAttr {
            attr_set,
            attr_clr,
            propagation,
            userns_fd,
        };

        // Call open_tree_attr via syscall (no libc wrapper)
        let result = unsafe {
            libc::syscall(
                467, // SYS_open_tree_attr
                dirfd,
                path_cstr.as_ptr(),
                flags,
                &host_attr as *const MountAttr,
                std::mem::size_of::<MountAttr>(),
            ) as i64
        };

        Ok(Self::libc_to_kernel(result))
    }
}
