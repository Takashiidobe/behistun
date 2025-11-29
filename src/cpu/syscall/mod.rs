mod directory_and_path_ops;
mod file_attributes_permissions;
mod file_io_basic;
mod file_metadata;
mod file_open_close;
mod futex_and_sync;
mod hostname_domain;
mod inotify_fanotify;
mod kernel_security;
mod memory_management;
mod polling_and_events;
mod posix_message_queues;
mod process_management;
mod random;
mod resource_limits;
mod scheduler_and_cpu_affinity;
mod signal_handling;
mod system_info;
mod sysvipc;
mod thread_and_tls;
mod timers_and_clocks;
mod user_identity;

use std::ffi::CString;

use anyhow::{Result, anyhow, bail};

use super::{Cpu, M68K_TLS_TCB_SIZE, TLS_DATA_PAD};
use crate::syscall::m68k_to_x86_64_syscall;

impl Cpu {
    /// Read syscall arguments from D1..D6 as a typed tuple.
    pub(super) fn get_args<T: FromRegs>(&self) -> T {
        T::from_regs(&self.data_regs[1..])
    }

    pub(super) fn handle_syscall(&mut self) -> Result<()> {
        let m68k_num = self.data_regs[0];
        let x86_num = m68k_to_x86_64_syscall(m68k_num).unwrap_or_default();

        // m68k Linux ABI: D0=syscall, D1-D5=args
        let result: i64 = match m68k_num {
            // exit(status) - no return
            1 => self.sys_exit(),

            // fork() - no pointers
            2 => self.sys_passthrough(x86_num, 0),

            // read(fd, buf, count) - buf is pointer
            3 => self.sys_read()?,

            // write(fd, buf, count) - buf is pointer
            4 => self.sys_write()?,

            // open(path, flags, mode) - path is pointer
            5 => self.sys_open()?,

            // close(fd) - no pointers
            6 => self.sys_passthrough(x86_num, 1),

            // waitpid(pid, status, options) - forward to wait4(pid,...,NULL)
            7 => self.sys_waitpid()?,

            // creat(path, mode) - path is pointer
            8 => self.sys_path1(x86_num, self.data_regs[2] as i64)?,

            // link(oldpath, newpath) - both pointers
            9 => self.sys_link()?,

            // unlink(path) - path is pointer
            10 => self.sys_path1(x86_num, 0)?,

            // execve(path, argv, envp) - replaces current process
            11 => self.sys_execve()?,

            // chdir(path) - path is pointer
            12 => self.sys_path1(x86_num, 0)?,

            // time(tloc) - tloc is pointer (can be NULL)
            13 => self.sys_time()?,

            // mknod(path, mode, dev) - path is pointer
            14 => self.sys_mknod()?,

            // chmod(path, mode) - path is pointer
            15 => self.sys_path1(x86_num, self.data_regs[2] as i64)?,

            // lchown(path, owner, group) - path is pointer
            16 => self.sys_chown(x86_num)?,

            // syscall 17 used to be break, doesn't exist anymore
            17 => -1,

            // syscall 18 is oldstat, can forward to stat
            18 => self.sys_stat(4)?,

            // lseek(fd, offset, whence) - no pointers
            19 => self.sys_passthrough(x86_num, 3),

            // getpid() - no pointers
            20 => self.sys_passthrough(x86_num, 0),

            // mount - complex, skip for now
            21 => bail!("mount not yet implemented"),

            // umount(target) - implemented as umount2(target, 0)
            22 => {
                let path_addr = self.data_regs[1] as usize;
                let path_cstr = self.guest_cstring(path_addr)?;
                let result = unsafe { libc::umount2(path_cstr.as_ptr(), 0) as i64 };
                Self::libc_to_kernel(result)
            }

            // setuid(uid) - no pointers
            23 => self.sys_passthrough(x86_num, 1),

            // getuid() - no pointers
            24 => self.sys_passthrough(x86_num, 0),

            // stime - skip for now
            25 => bail!("stime not yet implemented"),

            // ptrace - complex, skip for now
            26 => bail!("ptrace not yet implemented"),

            // alarm(seconds) - no pointers
            27 => self.sys_passthrough(x86_num, 1),

            // oldfstat - skip for now
            28 => bail!("oldfstat not yet implemented"),

            // pause() - no pointers
            29 => self.sys_passthrough(x86_num, 0),

            // utime(path, times) - path + struct pointer
            30 => self.sys_utime()?,

            // 31 was stty
            31 => -1,

            // 31 was gtty
            32 => -1,

            // access(path, mode) - path is pointer
            33 => self.sys_path1(x86_num, self.data_regs[2] as i64)?,

            // nice(incr)
            34 => bail!("nice not yet implemented"),

            // 35 was ftime
            35 => -1,

            // sync() - no pointers
            36 => self.sys_passthrough(x86_num, 0),

            // kill(pid, sig) - no pointers
            37 => self.sys_passthrough(x86_num, 2),

            // rename(old, new) - both pointers
            38 => self.sys_rename()?,

            // mkdir(path, mode) - path is pointer
            39 => self.sys_path1(x86_num, self.data_regs[2] as i64)?,

            // rmdir(path) - path is pointer
            40 => self.sys_path1(x86_num, 0)?,

            // dup(fd) - no pointers
            41 => self.sys_passthrough(x86_num, 1),

            // pipe(pipefd) - pointer to int[2]
            42 => self.sys_pipe()?,

            // times(buf) - pointer to struct tms
            43 => self.sys_times()?,

            // 44 was prof
            44 => -1,

            // brk(addr) - special handling
            45 => self.sys_brk()?,

            // setgid(gid) - no pointers
            46 => self.sys_passthrough(x86_num, 1),

            // getgid() - no pointers
            47 => self.sys_passthrough(x86_num, 0),

            // 48 is signal, complicated
            48 => bail!("signal not yet implemented"),

            // geteuid() - no pointers
            49 => self.sys_passthrough(x86_num, 0),

            // getegid() - no pointers
            50 => self.sys_passthrough(x86_num, 0),

            // acct(filename) - path pointer (can be NULL)
            51 => {
                let path = self.data_regs[1] as usize;
                if path == 0 {
                    self.sys_passthrough(x86_num, 1)
                } else {
                    self.sys_path1(x86_num, 0)?
                }
            }

            // umount2(target, flags) - path pointer
            52 => self.sys_path1(x86_num, self.data_regs[2] as i64)?,

            // 53 was lock
            53 => -1,

            // ioctl(fd, request, arg) - complex, passthrough for now
            54 => self.sys_passthrough(x86_num, 3),

            // fcntl(fd, cmd, arg) - mostly no pointers
            55 => self.sys_passthrough(x86_num, 3),

            // 56 was mpx
            56 => -1,

            // setpgid(pid, pgid) - no pointers
            57 => self.sys_passthrough(x86_num, 2),

            // 58 was ulimit
            58 => -1,

            // 59 was oldolduname
            59 => -1,

            // umask(mask) - no pointers
            60 => self.sys_passthrough(x86_num, 1),

            // chroot(path) - path pointer
            61 => self.sys_path1(x86_num, 0)?,

            // 62 ustat(dev, ubuf) - information about mounted fs. deprecated
            62 => bail!("ustat not yet implemented"),

            // dup2(old, new) - no pointers
            63 => self.sys_passthrough(x86_num, 2),

            // getppid() - no pointers
            64 => self.sys_passthrough(x86_num, 0),

            // getpgrp() - no pointers
            65 => self.sys_passthrough(x86_num, 0),

            // setsid() - no pointers
            66 => self.sys_passthrough(x86_num, 0),

            // sigaction
            67 => bail!("sigaction not yet implemented"),

            // sgetmask
            68 => bail!("sgetmask not yet implemented"),

            // ssetmask
            69 => bail!("ssetmask not yet implemented"),

            // setreuid(ruid, euid) - no pointers
            70 => self.sys_passthrough(x86_num, 2),

            // setregid(rgid, egid) - no pointers
            71 => self.sys_passthrough(x86_num, 2),

            // sigsuspend
            72 => bail!("sigsuspend not yet implemented"),

            // sigpending
            73 => bail!("sigpending not yet implemented"),

            // sethostname(name, len) - pointer
            74 => self.sys_sethostname()?,

            // setrlimit(resource, rlim) - pointer to struct
            75 => self.sys_setrlimit()?,

            // getrlimit(resource, rlim) - pointer to struct
            76 => self.sys_getrlimit()?,

            // getrusage(who, usage) - pointer to struct
            77 => self.sys_getrusage()?,

            // gettimeofday(tv, tz) - pointers
            78 => self.sys_gettimeofday()?,

            // settimeofday(tv, tz) - pointers
            79 => self.sys_settimeofday()?,

            // getgroups(size, list) - pointer
            80 => self.sys_getgroups()?,

            // setgroups(size, list) - pointer
            81 => self.sys_setgroups()?,

            // select(nfds, readfds, writefds, exceptfds, timeout) - pointers
            82 => self.sys_select()?,

            // symlink(target, linkpath) - both pointers
            83 => self.sys_symlink()?,

            // oldlstat not implemented
            84 => -1,

            // readlink(path, buf, size) - path + buf pointers
            85 => self.sys_readlink()?,

            // uselib, deprecated
            86 => -1,

            // swapon(path, flags) - path pointer
            87 => self.sys_path1(x86_num, self.data_regs[2] as i64)?,

            // reboot(magic, magic2, cmd, arg) - no pointers needed
            88 => self.sys_passthrough(x86_num, 4),

            // readdir, superseded by getdents
            89 => -1,

            // mmap - use new mmap2 style (syscall 90 is old_mmap on m68k)
            90 => self.sys_mmap()?,

            // munmap(addr, length) - no pointers (addr is value)
            91 => self.sys_passthrough(x86_num, 2),

            // truncate(path, length) - path pointer
            92 => self.sys_path1(x86_num, self.data_regs[2] as i64)?,

            // ftruncate(fd, length) - no pointers
            93 => self.sys_passthrough(x86_num, 2),

            // fchmod(fd, mode) - no pointers
            94 => self.sys_passthrough(x86_num, 2),

            // fchown(fd, owner, group) - no pointers
            95 => self.sys_passthrough(x86_num, 3),

            // getpriority(which, who) - no pointers
            96 => self.sys_passthrough(x86_num, 2),

            // setpriority(which, who, prio) - no pointers
            97 => self.sys_passthrough(x86_num, 3),

            // 98 was profil
            98 => -1,

            // statfs(path, buf) - path + struct pointer
            99 => self.sys_statfs()?,

            // fstatfs(fd, buf) - struct pointer
            100 => self.sys_fstatfs()?,

            // ioperm(from, num, turn_on) - no pointers
            101 => self.sys_passthrough(x86_num, 3),

            // socketcall
            102 => bail!("socketcall not yet implemented"),

            // syslog(type, buf, len) - buf pointer
            103 => self.sys_syslog()?,

            // setitimer(which, new, old) - struct pointers
            104 => self.sys_setitimer()?,

            // getitimer(which, curr) - struct pointer
            105 => self.sys_getitimer()?,

            // stat(path, buf) - path + struct pointer
            106 => self.sys_stat(x86_num)?,

            // lstat(path, buf) - path + struct pointer
            107 => self.sys_stat(x86_num)?,

            // fstat(fd, buf) - struct pointer
            108 => self.sys_fstat()?,

            // 109 was olduname
            109 => -1,

            // 109 was iopl
            110 => -1,

            // vhangup() - no pointers
            111 => self.sys_passthrough(x86_num, 0),

            // 112 was idle
            112 => -1,

            // 113 was vm86
            113 => -1,

            // wait4(pid, status, options, rusage) - pointers
            114 => self.sys_wait4()?,

            // swapoff(path) - path pointer
            115 => self.sys_path1(x86_num, 0)?,

            // sysinfo(info) - struct pointer
            116 => self.sys_sysinfo()?,

            // ipc(call, first, second, third, ptr, fifth)
            // Multiplexer that dispatches to individual IPC syscalls
            117 => self.sys_ipc()?,

            // fsync(fd) - no pointers
            118 => self.sys_passthrough(x86_num, 1),

            // sigreturn
            119 => bail!("sigreturn not yet implemented"),

            // clone(flags, stack, parent_tid, child_tid, tls)
            120 => self.sys_clone()?,

            // setdomainname(name, len) - pointer
            121 => {
                let name_addr = self.data_regs[1] as usize;
                let len = self.data_regs[2] as usize;
                let host_ptr = self
                    .memory
                    .guest_to_host(name_addr, len)
                    .ok_or_else(|| anyhow!("invalid domainname buffer"))?;
                unsafe { libc::syscall(x86_num as i64, host_ptr, len) }
            }

            // uname(buf) - struct pointer
            122 => self.sys_uname()?,

            // int cacheflush(unsigned long addr, int scope, int cache, unsigned long size);
            // doesn't need to do anything on an interpreter.
            123 => 0,

            // adjtimex(buf) - struct pointer
            124 => self.sys_adjtimex()?,

            // mprotect(addr, len, prot) - validates guest memory range
            125 => self.sys_mprotect()?,

            // sigprocmask - complex
            126 => bail!("sigprocmask not yet implemented"),

            // create_module - complex
            127 => bail!("create_module not yet implemented"),

            // init_module(module_image, len, param_values)
            128 => self.sys_init_module()?,

            // delete_module(name, flags) - path pointer
            129 => self.sys_path1(x86_num, self.data_regs[2] as i64)?,

            // 130, get kernel_syms
            130 => bail!("get_kernel_syms not yet implemented"),

            // 131, quotactl
            131 => bail!("quotactl not yet implemented"),

            // getpgid(pid) - no pointers
            132 => self.sys_passthrough(x86_num, 1),

            // fchdir(fd) - no pointers
            133 => self.sys_passthrough(x86_num, 1),

            // bdflush int bdflush(int func, long data);
            134 => bail!("bdflush not yet implemented"),

            // personality(persona) - no pointers
            135 => self.sys_passthrough(x86_num, 1),

            // personality (alternate number on m68k)
            136 => self.sys_passthrough(libc::SYS_personality as u32, 1),

            // 137 was afs syscall
            137 => -1,

            // setfsuid(uid) - no pointers
            138 => self.sys_passthrough(x86_num, 1),

            // setfsgid(gid) - no pointers
            139 => self.sys_passthrough(x86_num, 1),

            // _llseek(fd, offset_high, offset_low, result, whence) - result pointer
            140 => self.sys_llseek()?,

            // getdents(fd, dirp, count) - pointer - 32-bit dirent
            141 => self.sys_getdents32()?,

            // _newselect, forward to select
            142 => self.sys_select()?,

            // flock(fd, operation) - no pointers
            143 => self.sys_passthrough(x86_num, 2),

            // msync(addr, length, flags) - no pointers
            144 => self.sys_passthrough(x86_num, 3),

            // readv(fd, iov, iovcnt) - iov pointer
            145 => self.sys_readv()?,

            // writev(fd, iov, iovcnt) - iov pointer
            146 => self.sys_writev()?,

            // getsid(pid) - no pointers
            147 => self.sys_passthrough(x86_num, 1),

            // fdatasync(fd) - no pointers
            148 => self.sys_passthrough(x86_num, 1),

            // _sysctl(args) - deprecated
            149 => bail!("_sysctl not yet implemented"),

            // mlock(addr, len) - no pointers
            150 => self.sys_passthrough(x86_num, 2),

            // munlock(addr, len) - no pointers
            151 => self.sys_passthrough(x86_num, 2),

            // mlockall(flags) - no pointers
            152 => self.sys_passthrough(x86_num, 1),

            // munlockall() - no pointers
            153 => self.sys_passthrough(x86_num, 0),

            // sched_setparam(pid, param) - struct pointer
            154 => self.sys_sched_setparam()?,

            // sched_getparam(pid, param) - struct pointer
            155 => self.sys_sched_getparam()?,

            // sched_setscheduler(pid, policy, param) - struct pointer
            156 => self.sys_sched_setscheduler()?,

            // sched_getscheduler(pid) - no pointers
            157 => self.sys_passthrough(x86_num, 1),

            // sched_yield() - no pointers
            158 => self.sys_passthrough(x86_num, 0),

            // sched_get_priority_max(policy) - no pointers
            159 => self.sys_passthrough(x86_num, 1),

            // sched_get_priority_min(policy) - no pointers
            160 => self.sys_passthrough(x86_num, 1),

            // sched_rr_get_interval(pid, tp) - struct pointer
            161 => self.sys_sched_rr_get_interval()?,

            // nanosleep(req, rem) - struct pointers
            162 => self.sys_nanosleep()?,

            // mremap(old_addr, old_size, new_size, flags, new_addr) - no pointers
            163 => self.sys_passthrough(x86_num, 5),

            // setresuid(ruid, euid, suid) - no pointers
            164 => self.sys_passthrough(x86_num, 3),

            // getresuid(ruid, euid, suid) - pointers
            165 => self.sys_getresuid()?,

            // getpagesize() - m68k 166, derive from host
            166 => self.sys_getpagesize()?,

            // query_module(name, which, buf, bufsize, ret) - deprecated
            167 => bail!("query_module not yet implemented"),

            // poll(fds, nfds, timeout) - pointer
            168 => self.sys_poll()?,

            // nfsservctl - removed from kernel
            169 => bail!("nfsservctl not yet implemented"),

            // setresgid(rgid, egid, sgid) - no pointers
            170 => self.sys_passthrough(x86_num, 3),

            // getresgid(rgid, egid, sgid) - pointers
            171 => self.sys_getresgid()?,

            // prctl(option, arg2, arg3, arg4, arg5) - m68k 172
            172 => self.sys_prctl()?,

            // rt_sigreturn - signal handling
            173 => bail!("rt_sigreturn not yet implemented"),

            // rt_sigaction(sig, act, oact, sigsetsize) - m68k 174
            174 => self.sys_passthrough(x86_num, 4),

            // rt_sigprocmask(how, set, oldset, sigsetsize) - m68k 175
            175 => self.sys_passthrough(x86_num, 4),

            // rt_sigpending(set, sigsetsize) - m68k 176
            176 => self.sys_passthrough(x86_num, 2),

            // rt_sigtimedwait(set, info, timeout, sigsetsize)
            177 => bail!("rt_sigtimedwait not yet implemented"),

            // rt_sigqueueinfo(tgid, sig, info)
            178 => bail!("rt_sigqueueinfo not yet implemented"),

            // rt_sigsuspend(mask, sigsetsize)
            179 => bail!("rt_sigsuspend not yet implemented"),

            // pread64(fd, buf, count, offset) - buf pointer
            180 => self.sys_pread64()?,

            // pwrite64(fd, buf, count, offset) - buf pointer
            181 => self.sys_pwrite64()?,

            // chown(path, owner, group) - path pointer
            182 => self.sys_chown(x86_num)?,

            // getcwd(buf, size) - buf pointer
            183 => self.sys_getcwd()?,

            // capget(hdrp, datap) - m68k 184
            184 => self.sys_capget()?,

            // capset(hdrp, datap) - m68k 185
            185 => self.sys_capset()?,

            // sigaltstack(ss, old_ss)
            186 => bail!("sigaltstack not yet implemented"),

            // sendfile(out_fd, in_fd, offset, count) - offset is pointer
            187 => self.sys_sendfile()?,

            // getpmsg - STREAMS, not implemented in Linux
            188 => bail!("getpmsg not yet implemented"),

            // putpmsg - STREAMS, not implemented in Linux
            189 => bail!("putpmsg not yet implemented"),

            // vfork() - convert to fork to avoid memory corruption
            // vfork shares memory with parent, which breaks our execve implementation
            // that modifies self.memory. Converting to fork gives us copy-on-write.
            190 => {
                let result = unsafe { libc::fork() as i64 };
                Self::libc_to_kernel(result)
            }

            // ugetrlimit(resource, rlim) - m68k only, same as getrlimit
            191 => self.sys_getrlimit()?,

            // mmap2(addr, length, prot, flags, fd, pgoffset) - offset is in pages
            192 => self.sys_mmap2()?,

            // truncate64(path, length) - path is pointer
            193 => self.sys_truncate()?,

            // ftruncate64(fd, length) - maps to ftruncate on x86_64
            194 => self.sys_passthrough(x86_num, 2),

            // stat64(path, buf) - path pointer, struct pointer
            195 => self.sys_stat(x86_num)?,

            // lstat64(path, buf) - path pointer, struct pointer
            196 => self.sys_stat(x86_num)?,

            // fstat64(fd, buf) - struct pointer
            197 => self.sys_fstat()?,

            // chown32(path, owner, group) - path pointer
            198 => self.sys_chown(x86_num)?,

            // getuid32() -> forward to getuid
            199 => self.sys_passthrough(x86_num, 0),

            // getgid32() -> forward to getgid
            200 => self.sys_passthrough(x86_num, 0),

            // geteuid32() -> forward to geteuid
            201 => self.sys_passthrough(x86_num, 0),

            // getegid32() -> forward to getegid
            202 => self.sys_passthrough(x86_num, 0),

            // setreuid32(ruid, euid)
            203 => self.sys_passthrough(x86_num, 2),

            // setregid32(rgid, egid)
            204 => self.sys_passthrough(x86_num, 2),

            // getgroups32(size, list)
            205 => self.sys_getgroups()?,

            // setgroups32(size, list)
            206 => self.sys_setgroups()?,

            // fchown32(fd, owner, group)
            207 => self.sys_passthrough(x86_num, 3),

            // setresuid32(ruid, euid, suid)
            208 => self.sys_passthrough(x86_num, 3),

            // getresuid32(ruid, euid, suid)
            209 => self.sys_getresuid()?,

            // setresgid32(rgid, egid, sgid)
            210 => self.sys_passthrough(x86_num, 3),

            // getresgid32(rgid, egid, sgid)
            211 => self.sys_getresgid()?,

            // lchown32(path, owner, group)
            212 => self.sys_chown(x86_num)?,

            // setuid32(uid)
            213 => self.sys_passthrough(x86_num, 1),

            // setgid32(gid)
            214 => self.sys_passthrough(x86_num, 1),

            // setfsuid32(uid)
            215 => self.sys_passthrough(x86_num, 1),

            // setfsgid32(gid)
            216 => self.sys_passthrough(x86_num, 1),

            // pivot_root(new_root, put_old)
            217 => bail!("pivot_root not yet implemented"),

            // getdents64(fd, dirp, count) - 64-bit dirent64
            220 => self.sys_getdents64()?,

            // fcntl64(fd, cmd, arg)
            221 => self.sys_passthrough(x86_num, 3),

            // tkill(tid, sig)
            222 => bail!("tkill not yet implemented"),

            // setxattr(path, name, value, size, flags)
            223 => self.sys_setxattr()?,

            // lsetxattr(path, name, value, size, flags)
            224 => self.sys_lsetxattr()?,

            // fsetxattr(fd, name, value, size, flags)
            225 => self.sys_fsetxattr()?,

            // getxattr(path, name, value, size)
            226 => self.sys_getxattr()?,

            // lgetxattr(path, name, value, size)
            227 => self.sys_lgetxattr()?,

            // fgetxattr(fd, name, value, size)
            228 => self.sys_fgetxattr()?,

            // listxattr(path, list, size)
            229 => self.sys_listxattr()?,

            // llistxattr(path, list, size)
            230 => self.sys_llistxattr()?,

            // flistxattr(fd, list, size)
            231 => self.sys_flistxattr()?,

            // removexattr(path, name)
            232 => self.sys_removexattr()?,

            // lremovexattr(path, name)
            233 => self.sys_lremovexattr()?,

            // fremovexattr(fd, name)
            234 => self.sys_fremovexattr()?,

            // futex(uaddr, op, val, timeout, uaddr2, val3) - m68k 235
            235 => self.sys_futex()?,

            // sendfile64(out_fd, in_fd, offset, count) - offset is pointer
            236 => self.sys_sendfile()?,

            // mincore(addr, length, vec)
            237 => self.sys_mincore()?,

            // madvise(addr, length, advice)
            238 => bail!("madvise not yet implemented"),

            // fcntl64(fd, cmd, arg)
            239 => self.sys_passthrough(x86_num, 3),

            // readahead(fd, offset, count)
            240 => bail!("readahead not yet implemented"),

            // io_setup(nr_events, ctx)
            241 => bail!("io_setup not yet implemented"),

            // io_destroy(ctx)
            242 => bail!("io_destroy not yet implemented"),

            // io_getevents(ctx, min_nr, nr, events, timeout)
            243 => bail!("io_getevents not yet implemented"),

            // io_submit(ctx, nr, iocbpp)
            244 => bail!("io_submit not yet implemented"),

            // io_cancel(ctx, iocb, result)
            245 => bail!("io_cancel not yet implemented"),

            // fadvise64(fd, offset, len, advice)
            246 => self.sys_passthrough(x86_num, 4),

            // exit_group(status)
            247 => self.sys_passthrough(x86_num, 1),

            // lookup_dcookie(cookie, buffer, len)
            248 => bail!("lookup_dcookie not yet implemented"),

            // epoll_create(size)
            249 => self.sys_passthrough(x86_num, 1),

            // epoll_ctl(epfd, op, fd, event) - m68k 250
            250 => self.sys_passthrough(x86_num, 4),

            // epoll_wait(epfd, events, maxevents, timeout) - m68k 251
            251 => self.sys_passthrough(x86_num, 4),

            // remap_file_pages(addr, size, prot, pgoff, flags)
            252 => bail!("remap_file_pages not yet implemented"),

            // set_tid_address(tidptr) - pointer
            253 => self.sys_passthrough(x86_num, 1),

            // timer_create(clockid, sevp, timerid) - m68k 254
            254 => self.sys_passthrough(x86_num, 3),

            // timer_settime(timerid, flags, new_value, old_value) - m68k 255
            255 => self.sys_passthrough(x86_num, 4),

            // timer_gettime(timerid, curr_value) - m68k 256
            256 => self.sys_passthrough(x86_num, 2),

            // timer_delete(timerid) - m68k 257
            257 => self.sys_passthrough(x86_num, 1),

            // timer_getoverrun(timerid)
            258 => bail!("timer_getoverrun not yet implemented"),

            // clock_settime(clockid, timespec)
            259 => self.sys_clock_settime()?,

            // clock_gettime(clockid, timespec)
            260 => self.sys_clock_gettime()?,

            // clock_getres(clockid, timespec)
            261 => self.sys_clock_getres()?,

            // clock_nanosleep(clockid, flags, request, remain)
            262 => self.sys_clock_nanosleep()?,

            // statfs64(path, buf) - path pointer, struct pointer
            263 => self.sys_statfs()?,

            // fstatfs64(fd, buf) - struct pointer
            264 => self.sys_fstatfs()?,

            // tgkill(tgid, tid, sig)
            265 => self.sys_passthrough(x86_num, 3),

            // utimes(filename, times) - path pointer, timeval array pointer
            266 => self.sys_utimes()?,

            // fadvise64_64(fd, offset, len, advice) - m68k only
            267 => self.sys_passthrough(x86_num, 4),

            // mbind(addr, len, mode, nodemask, maxnode, flags)
            268 => bail!("mbind not yet implemented"),

            // get_mempolicy(policy, nodemask, maxnode, addr, flags)
            269 => bail!("get_mempolicy not yet implemented"),

            // set_mempolicy(mode, nodemask, maxnode)
            270 => bail!("set_mempolicy not yet implemented"),

            // mq_open(name, oflag, mode, attr) - m68k 271
            271 => self.sys_mq_open()?,

            // mq_unlink(name) - m68k 272
            272 => self.sys_mq_unlink()?,

            // mq_timedsend(mqdes, msg_ptr, msg_len, msg_prio, abs_timeout) - m68k 273
            273 => self.sys_mq_timedsend()?,

            // mq_timedreceive(mqdes, msg_ptr, msg_len, msg_prio, abs_timeout) - m68k 274
            274 => self.sys_mq_timedreceive()?,

            // mq_notify(mqdes, notification)
            275 => bail!("mq_notify not yet implemented"),

            // mq_getsetattr(mqdes, newattr, oldattr) - m68k 276
            276 => self.sys_mq_getsetattr()?,

            // waitid(idtype, id, infop, options) - m68k 277
            277 => self.sys_waitid()?,

            // add_key(type, description, payload, plen, keyring)
            279 => bail!("add_key not yet implemented"),

            // request_key(type, description, callout_info, keyring)
            280 => bail!("request_key not yet implemented"),

            // keyctl(cmd, arg2, arg3, arg4, arg5)
            281 => bail!("keyctl not yet implemented"),

            // ioprio_set(which, who, ioprio)
            282 => bail!("ioprio_set not yet implemented"),

            // ioprio_get(which, who)
            283 => bail!("ioprio_get not yet implemented"),

            // inotify_init() - m68k 284
            284 => self.sys_passthrough(x86_num, 0),

            // inotify_add_watch(fd, path, mask) - m68k 285
            285 => self.sys_inotify_add_watch()?,

            // inotify_rm_watch(fd, wd) - m68k 286
            286 => self.sys_passthrough(x86_num, 2),

            // migrate_pages(pid, maxnode, old_nodes, new_nodes)
            287 => bail!("migrate_pages not yet implemented"),

            // openat(dirfd, path, flags, mode)
            288 => self.sys_openat()?,

            // mkdirat(dirfd, path, mode) - m68k 289
            289 => self.sys_mkdirat()?,

            // fchownat(dirfd, path, owner, group, flags) - m68k 291
            291 => self.sys_fchownat()?,

            // mknodat(dirfd, path, mode, dev) - m68k 290
            290 => self.sys_mknodat()?,

            // futimesat(dirfd, path, times) - m68k 292
            292 => self.sys_futimesat()?,

            // fstatat64(dirfd, path, buf, flags) - m68k 293
            293 => self.sys_fstatat64()?,

            // unlinkat(dirfd, path, flags) - m68k 294
            294 => self.sys_unlinkat()?,

            // renameat(olddirfd, oldpath, newdirfd, newpath) - m68k 295
            295 => self.sys_renameat()?,

            // linkat(olddirfd, oldpath, newdirfd, newpath, flags) - m68k 296
            296 => self.sys_linkat()?,

            // symlinkat(target, newdirfd, linkpath) - m68k 297
            297 => self.sys_symlinkat()?,

            // readlinkat(dirfd, path, buf, bufsiz) - m68k 298
            298 => self.sys_readlinkat()?,

            // fchmodat(dirfd, path, mode, flags) - m68k 299
            299 => self.sys_fchmodat()?,

            // faccessat(dirfd, path, mode, flags) - m68k 300
            300 => self.sys_faccessat()?,

            // pselect6(nfds, readfds, writefds, exceptfds, timeout, sigmask) - m68k 301
            301 => self.sys_passthrough(x86_num, 6),

            // ppoll(fds, nfds, timeout, sigmask, sigsetsize) - m68k 302
            302 => self.sys_passthrough(x86_num, 5),

            // unshare(flags)
            303 => bail!("unshare not yet implemented"),

            // set_robust_list(head, len)
            304 => bail!("set_robust_list not yet implemented"),

            // get_robust_list(pid, head, len)
            305 => bail!("get_robust_list not yet implemented"),

            // splice(fd_in, off_in, fd_out, off_out, len, flags) - m68k 306
            306 => self.sys_splice()?,

            // sync_file_range(fd, offset, nbytes, flags) - m68k 307
            307 => self.sys_passthrough(x86_num, 4),

            // tee(fd_in, fd_out, len, flags) - m68k 308
            308 => self.sys_passthrough(x86_num, 4),

            // vmsplice(fd, iov, nr_segs, flags) - m68k 309
            309 => self.sys_vmsplice()?,

            // move_pages(pid, count, pages, nodes, status, flags)
            310 => bail!("move_pages not yet implemented"),

            // sched_setaffinity(pid, cpusetsize, mask)
            311 => bail!("sched_setaffinity not yet implemented"),

            // sched_getaffinity(pid, cpusetsize, mask) - m68k 312
            312 => self.sys_passthrough(x86_num, 3),

            // kexec_load(entry, nr_segments, segments, flags)
            313 => bail!("kexec_load not yet implemented"),

            // getcpu(cpu, node, tcache) - m68k 314
            314 => self.sys_getcpu()?,

            // epoll_pwait(epfd, events, maxevents, timeout, sigmask, sigsetsize)
            315 => bail!("epoll_pwait not yet implemented"),

            // utimensat(dirfd, path, times, flags) - m68k 316
            316 => self.sys_utimensat()?,

            // signalfd(fd, mask, flags) - m68k 317
            317 => self.sys_signalfd()?,

            // timerfd_create(clockid, flags) - m68k 318
            318 => self.sys_passthrough(x86_num, 2),

            // eventfd(initval) - m68k 319
            319 => self.sys_passthrough(x86_num, 1),

            // fallocate(fd, mode, offset, len) - m68k 320
            320 => self.sys_passthrough(x86_num, 4),

            // timerfd_settime(fd, flags, new_value, old_value) - m68k 321
            321 => self.sys_timerfd_settime()?,

            // timerfd_gettime(fd, curr_value) - m68k 322
            322 => self.sys_timerfd_gettime()?,

            // signalfd4(fd, mask, sizemask, flags) - m68k 323
            323 => self.sys_signalfd4()?,

            // eventfd2(initval, flags) - m68k 324
            324 => self.sys_passthrough(x86_num, 2),

            // epoll_create1(flags) - m68k 325
            325 => self.sys_passthrough(x86_num, 1),

            // dup3(oldfd, newfd, flags) - m68k 326
            326 => self.sys_passthrough(x86_num, 3),

            // pipe2(pipefd, flags) - m68k 327
            327 => self.sys_pipe2()?,

            // inotify_init1(flags) - m68k 328
            328 => self.sys_passthrough(x86_num, 1),

            // preadv(fd, iov, iovcnt, pos_l, pos_h)
            329 => self.sys_preadv()?,

            // pwritev(fd, iov, iovcnt, pos_l, pos_h)
            330 => self.sys_pwritev()?,

            // rt_tgsigqueueinfo(tgid, tid, sig, info)
            331 => bail!("rt_tgsigqueueinfo not yet implemented"),

            // perf_event_open(attr, pid, cpu, group_fd, flags)
            332 => bail!("perf_event_open not yet implemented"),

            // get_thread_area()
            333 => self.sys_read_tp()?,

            // prlimit64(pid, resource, new_limit, old_limit) - m68k
            339 => self.sys_prlimit64()?,

            // name_to_handle_at(dirfd, name, handle, mnt_id, flags)
            340 => bail!("name_to_handle_at not yet implemented"),

            // open_by_handle_at(mountdirfd, handle, flags)
            341 => bail!("open_by_handle_at not yet implemented"),

            // clock_adjtime(clk_id, buf)
            342 => self.sys_clock_adjtime()?,

            // syncfs(fd) - m68k
            343 => self.sys_passthrough(x86_num, 1),

            // setns(fd, nstype)
            344 => bail!("setns not yet implemented"),

            // process_vm_readv(pid, local_iov, liovcnt, remote_iov, riovcnt, flags)
            345 => bail!("process_vm_readv not yet implemented"),

            // process_vm_writev(pid, local_iov, liovcnt, remote_iov, riovcnt, flags)
            346 => bail!("process_vm_writev not yet implemented"),

            // kcmp(pid1, pid2, type, idx1, idx2)
            347 => bail!("kcmp not yet implemented"),

            // finit_module(fd, param_values, flags)
            348 => bail!("finit_module not yet implemented"),

            // sched_setattr(pid, attr, flags)
            349 => bail!("sched_setattr not yet implemented"),

            // sched_getattr(pid, attr, size, flags)
            350 => bail!("sched_getattr not yet implemented"),

            // set_thread_area(addr)
            334 => self.set_thread_area()?,

            // renameat2(olddirfd, oldpath, newdirfd, newpath, flags) - m68k
            351 => self.sys_renameat2()?,

            // getrandom(buf, buflen, flags) - m68k
            352 => self.sys_getrandom()?,

            // memfd_create(name, flags) - m68k
            353 => self.sys_memfd_create()?,

            // bpf(cmd, attr, size)
            354 => bail!("bpf not yet implemented"),

            // execveat(dirfd, pathname, argv, envp, flags)
            355 => bail!("execveat not yet implemented"),

            // atomic_cmpxchg_32(uaddr, oldval, newval) - m68k
            335 => self.sys_atomic_cmpxchg_32()?,

            // atomic_barrier() - m68k
            336 => self.sys_atomic_barrier()?,

            // fanotify_init(flags, event_f_flags)
            337 => bail!("fanotify_init not yet implemented"),

            // fanotify_mark(fd, flags, mask, dirfd, pathname)
            338 => bail!("fanotify_mark not yet implemented"),

            // Socket syscalls (m68k uses separate syscalls, not socketcall)
            // socket(domain, type, protocol) - no pointers - m68k 356
            356 => self.sys_passthrough(x86_num, 3),

            // socketpair(domain, type, protocol, sv) - sv is pointer - m68k 357
            357 => self.sys_socketpair()?,

            // bind(sockfd, addr, addrlen) - addr is pointer - m68k 358
            358 => self.sys_socket_addr(x86_num)?,

            // connect(sockfd, addr, addrlen) - addr is pointer - m68k 359
            359 => self.sys_socket_addr(x86_num)?,

            // listen(sockfd, backlog) - no pointers - m68k 360
            360 => self.sys_passthrough(x86_num, 2),

            // accept4(sockfd, addr, addrlen, flags) - addr/addrlen pointers - m68k 361
            361 => self.sys_accept4()?,

            // getsockopt(sockfd, level, optname, optval, optlen) - pointers - m68k 362
            362 => self.sys_getsockopt()?,

            // setsockopt(sockfd, level, optname, optval, optlen) - optval pointer - m68k 363
            363 => self.sys_setsockopt()?,

            // getsockname(sockfd, addr, addrlen) - pointers - m68k 364
            364 => self.sys_getsockname()?,

            // getpeername(sockfd, addr, addrlen) - pointers - m68k 365
            365 => self.sys_getsockname()?,

            // sendto(sockfd, buf, len, flags, dest_addr, addrlen) - m68k 366
            366 => self.sys_sendto()?,

            // sendmsg(sockfd, msg, flags) - complex msghdr structure - m68k 367
            367 => self.sys_sendmsg()?,

            // recvfrom(sockfd, buf, len, flags, src_addr, addrlen) - m68k 368
            368 => self.sys_recvfrom()?,

            // recvmsg(sockfd, msg, flags) - complex msghdr structure - m68k 369
            369 => self.sys_recvmsg()?,

            // shutdown(sockfd, how) - no pointers - m68k 370
            370 => self.sys_passthrough(x86_num, 2),

            // recvmmsg(sockfd, msgvec, vlen, flags, timeout)
            371 => bail!("recvmmsg not yet implemented"),

            // sendmmsg(sockfd, msgvec, vlen, flags)
            372 => bail!("sendmmsg not yet implemented"),

            // userfaultfd(flags)
            373 => bail!("userfaultfd not yet implemented"),

            // membarrier(cmd, flags, cpu_id)
            374 => bail!("membarrier not yet implemented"),

            // mlock2(addr, len, flags) - m68k 375
            375 => self.sys_passthrough(x86_num, 3),

            // copy_file_range(fd_in, off_in, fd_out, off_out, len, flags) - m68k 376
            376 => self.sys_copy_file_range()?,

            // preadv2(fd, iov, iovcnt, offset, flags)
            377 => bail!("preadv2 not yet implemented"),

            // pwritev2(fd, iov, iovcnt, offset, flags)
            378 => bail!("pwritev2 not yet implemented"),

            // statx(dirfd, pathname, flags, mask, statxbuf) - m68k 379
            379 => self.sys_statx()?,

            // seccomp(operation, flags, args)
            380 => bail!("seccomp not yet implemented"),

            // pkey_mprotect(addr, len, prot, pkey) - m68k 381
            381 => self.sys_pkey_mprotect()?,

            // pkey_alloc(flags, access_rights) - m68k 382
            382 => self.sys_pkey_alloc()?,

            // pkey_free(pkey) - m68k 383
            383 => self.sys_pkey_free()?,

            // rseq(rseq, rseq_len, flags, sig) - m68k 384
            384 => self.sys_passthrough(x86_num, 4),

            // semget(key, nsems, semflg) - m68k 393
            393 => self.sys_passthrough(x86_num, 3),

            // semctl(semid, semnum, cmd, arg) - m68k 394
            394 => self.sys_semctl()?,

            // shmget(key, size, shmflg) - m68k 395
            395 => self.sys_passthrough(x86_num, 3),

            // shmctl(shmid, cmd, buf) - m68k 396
            396 => self.sys_shmctl()?,

            // shmat(shmid, shmaddr, shmflg) - m68k 397
            397 => self.sys_shmat()?,

            // shmdt(shmaddr) - m68k 398
            398 => self.sys_shmdt()?,

            // msgget(key, msgflg) - m68k 399
            399 => self.sys_passthrough(x86_num, 2),

            // msgsnd(msqid, msgp, msgsz, msgflg) - m68k 400
            400 => self.sys_msgsnd()?,

            // msgrcv(msqid, msgp, msgsz, msgtyp, msgflg) - m68k 401
            401 => self.sys_msgrcv()?,

            // msgctl(msqid, cmd, buf) - m68k 402
            402 => self.sys_msgctl()?,

            // clock_gettime(clockid, timespec)
            403 => self.sys_clock_gettime()?,

            // clock_settime64(clockid, timespec) -> clock_settime
            404 => self.sys_clock_settime()?,

            // clock_adjtime64(clk_id, buf)
            405 => self.sys_clock_adjtime()?,

            // clock_getres_time64(clockid, timespec) -> clock_getres
            406 => self.sys_clock_getres()?,

            // clock_nanosleep_time64(clockid, flags, request, remain) -> clock_nanosleep
            407 => self.sys_clock_nanosleep()?,

            // timer_gettime64(timerid, curr_value) -> timer_gettime
            408 => self.sys_passthrough(x86_num, 2),

            // timer_settime64(timerid, flags, new_value, old_value) -> timer_settime
            409 => self.sys_passthrough(x86_num, 4),

            // timerfd_gettime64(fd, curr_value) - m68k 410
            410 => self.sys_timerfd_gettime()?,

            // timerfd_settime64(fd, flags, new_value, old_value) - m68k 411
            411 => self.sys_timerfd_settime()?,

            // utimensat_time64(dirfd, path, times, flags) -> utimensat
            412 => self.sys_utimensat()?,

            // pselect6_time64(nfds, readfds, writefds, exceptfds, timeout, sigmask)
            413 => self.sys_pselect6()?,

            // ppoll_time64(fds, nfds, timeout, sigmask, sigsetsize)
            414 => bail!("ppoll_time64 not yet implemented"),

            // io_pgetevents_time64(ctx, min_nr, nr, events, timeout, sig)
            416 => bail!("io_pgetevents_time64 not yet implemented"),

            // recvmmsg_time64(sockfd, msgvec, vlen, flags, timeout)
            417 => bail!("recvmmsg_time64 not yet implemented"),

            // mq_timedsend_time64 - m68k 418 (same as mq_timedsend, already handles 64-bit time_t)
            418 => self.sys_mq_timedsend()?,

            // mq_timedreceive_time64 - m68k 419 (same as mq_timedreceive, already handles 64-bit time_t)
            419 => self.sys_mq_timedreceive()?,

            // semtimedop_time64(semid, sops, nsops, timeout)
            420 => bail!("semtimedop_time64 not yet implemented"),

            // rt_sigtimedwait_time64(set, info, timeout, sigsetsize)
            421 => bail!("rt_sigtimedwait_time64 not yet implemented"),

            // futex_time64(uaddr, op, val, timeout, uaddr2, val3)
            422 => bail!("futex_time64 not yet implemented"),

            // sched_rr_get_interval_time64(pid, tp)
            423 => bail!("sched_rr_get_interval_time64 not yet implemented"),

            // pidfd_send_signal(pidfd, sig, info, flags)
            424 => bail!("pidfd_send_signal not yet implemented"),

            // io_uring_setup(entries, params)
            425 => bail!("io_uring_setup not yet implemented"),

            // io_uring_enter(fd, to_submit, min_complete, flags, sig)
            426 => bail!("io_uring_enter not yet implemented"),

            // io_uring_register(fd, opcode, arg, nr_args)
            427 => bail!("io_uring_register not yet implemented"),

            // open_tree(dirfd, pathname, flags)
            428 => bail!("open_tree not yet implemented"),

            // move_mount(from_dirfd, from_pathname, to_dirfd, to_pathname, flags)
            429 => bail!("move_mount not yet implemented"),

            // fsopen(fsname, flags)
            430 => bail!("fsopen not yet implemented"),

            // fsconfig(fd, cmd, key, value, aux)
            431 => bail!("fsconfig not yet implemented"),

            // fsmount(fd, flags, attr_flags)
            432 => bail!("fsmount not yet implemented"),

            // fspick(dirfd, pathname, flags)
            433 => bail!("fspick not yet implemented"),

            // pidfd_open(pid, flags)
            434 => bail!("pidfd_open not yet implemented"),

            // clone3(cl_args, size)
            435 => bail!("clone3 not yet implemented"),

            // close_range(first, last, flags)
            436 => self.sys_passthrough(x86_num, 3),

            // openat2(dirfd, path, how, size) - extended openat with open_how struct
            437 => self.sys_openat2()?,

            // pidfd_getfd(pidfd, targetfd, flags)
            438 => bail!("pidfd_getfd not yet implemented"),

            // faccessat2(dirfd, pathname, mode, flags)
            439 => bail!("faccessat2 not yet implemented"),

            // process_madvise(pidfd, iovec, vlen, advice, flags)
            440 => bail!("process_madvise not yet implemented"),

            // epoll_pwait2(epfd, events, maxevents, timeout, sigmask, sigsetsize)
            441 => bail!("epoll_pwait2 not yet implemented"),

            // mount_setattr(dirfd, pathname, flags, attr, size)
            442 => bail!("mount_setattr not yet implemented"),

            // quotactl_fd(fd, cmd, id, addr)
            443 => bail!("quotactl_fd not yet implemented"),

            // landlock_create_ruleset(attr, size, flags)
            444 => self.sys_landlock_create_ruleset()?,

            // landlock_add_rule(ruleset_fd, rule_type, rule_attr, flags)
            445 => self.sys_landlock_add_rule()?,

            // landlock_restrict_self(ruleset_fd, flags)
            446 => self.sys_landlock_restrict_self()?,

            // process_mrelease(pidfd, flags)
            448 => bail!("process_mrelease not yet implemented"),

            // futex_waitv(waiters, nr_futexes, flags, timeout, clockid)
            449 => bail!("futex_waitv not yet implemented"),

            // set_mempolicy_home_node(addr, len, home_node, flags)
            450 => bail!("set_mempolicy_home_node not yet implemented"),

            // cachestat(fd, cstat_range, cstat, flags)
            451 => bail!("cachestat not yet implemented"),

            // fchmodat2(dirfd, path, mode, flags) - m68k 452
            452 => self.sys_fchmodat2()?,

            // map_shadow_stack(addr, size, flags)
            453 => bail!("map_shadow_stack not yet implemented"),

            // futex_wake(uaddr, nr_wake, mask, flags)
            454 => bail!("futex_wake not yet implemented"),

            // futex_wait(uaddr, val, mask, flags, timeout, clockid)
            455 => bail!("futex_wait not yet implemented"),

            // futex_requeue(uaddr, uaddr2, nr_wake, nr_requeue, cmpval, flags)
            456 => bail!("futex_requeue not yet implemented"),

            // statmount(mnt_id, buf, bufsize, flags)
            457 => bail!("statmount not yet implemented"),

            // listmount(mnt_id, buf, bufsize, flags)
            458 => bail!("listmount not yet implemented"),

            // lsm_get_self_attr(attr, ctx, size, flags)
            459 => bail!("lsm_get_self_attr not yet implemented"),

            // lsm_set_self_attr(attr, ctx, size, flags)
            460 => bail!("lsm_set_self_attr not yet implemented"),

            // lsm_list_modules(ids, size, flags)
            461 => bail!("lsm_list_modules not yet implemented"),

            // mseal(addr, len, flags). Unsupported on 32-bit linux, so return -EPERM
            462 => self.sys_mseal()?,

            // setxattrat(dirfd, path, name, value, size, flags)
            463 => self.sys_setxattrat()?,

            // getxattrat(dirfd, path, name, value, size)
            464 => self.sys_getxattrat()?,

            // listxattrat(dirfd, path, args, atflags)
            465 => self.sys_listxattrat()?,

            // removexattrat(dirfd, path, name, atflags)
            466 => self.sys_removexattrat()?,

            // open_tree_attr(dirfd, path, flags, attr, size) - m68k 467
            467 => self.sys_open_tree_attr()?,

            // file_getattr(dirfd, path, *fsx, size, at_flags)
            468 => bail!("file_getattr not yet implemented"),

            // file_setattr(dirfd, path, *fsx, size, at_flags)
            469 => bail!("file_setattr not yet implemented"),

            // For syscalls with no pointer args, passthrough directly
            syscall_num => bail!("Unsupported syscall number: {syscall_num}"),
        };

        self.data_regs[0] = result as u32;
        Ok(())
    }

    /// Set thread area
    fn set_thread_area(&mut self) -> Result<i64> {
        let tls_addr = self.data_regs[1] as usize;
        self.ensure_tls_range(tls_addr)?;
        if self.tls_memsz > 0 {
            let new_start = tls_addr
                .checked_sub(M68K_TLS_TCB_SIZE)
                .ok_or_else(|| anyhow!("new TLS base underflow"))?;
            if self.memory.covers_range(new_start, self.tls_memsz) {
                let zeros = vec![0u8; self.tls_memsz];
                self.memory.write_data(new_start, &zeros)?;
            }
        }
        if self.tls_base != 0 && self.tls_base as usize != tls_addr {
            let old_start =
                self.tls_base
                    .checked_sub(M68K_TLS_TCB_SIZE as u32)
                    .ok_or_else(|| anyhow!("old TLS base underflow"))? as usize;
            let new_start = tls_addr
                .checked_sub(M68K_TLS_TCB_SIZE)
                .ok_or_else(|| anyhow!("new TLS base underflow"))?;
            let copy_len = self.tls_memsz.min(M68K_TLS_TCB_SIZE + TLS_DATA_PAD);
            if copy_len > 0 && self.memory.covers_range(new_start, copy_len) {
                if self.memory.covers_range(old_start, copy_len) && self.tls_initialized {
                    let snapshot = self.memory.read_data(old_start, copy_len)?.to_vec();
                    self.memory.write_data(new_start, &snapshot)?;
                } else {
                    let zeros = vec![0u8; copy_len];
                    self.memory.write_data(new_start, &zeros)?;
                }
            }
        }
        self.tls_base = tls_addr as u32;
        self.tls_initialized = true;
        // Note: Don't modify A0 or A6 - A6 is the frame pointer!
        // The syscall return value (0 for success) goes in D0.
        Ok(0)
    }

    /// Ensure the TLS region around the thread pointer is backed by memory.
    pub(super) fn ensure_tls_range(&mut self, thread_ptr: usize) -> Result<()> {
        let end = thread_ptr
            .checked_add(TLS_DATA_PAD)
            .ok_or_else(|| anyhow!("thread pointer overflow"))?;

        // TLS is expected to live on the heap; grow the heap segment if needed.
        let heap_len = self
            .memory
            .segments()
            .iter()
            .find(|s| s.vaddr == self.heap_segment_base)
            .map(|s| s.len())
            .ok_or_else(|| anyhow!("heap segment not found"))?;
        let current_end = self.heap_segment_base + heap_len;
        if end > current_end {
            // Keep a small guard below the stack.
            let guard: usize = 0x1000;
            if end + guard > self.stack_base {
                bail!("TLS region would overlap the stack");
            }
            let new_len = end
                .checked_sub(self.heap_segment_base)
                .ok_or_else(|| anyhow!("TLS end before heap base"))?;
            self.memory
                .resize_segment(self.heap_segment_base, new_len)?;
        }

        Ok(())
    }

    /// Convert libc syscall result (-1 + errno) to kernel ABI (-errno).
    /// libc::syscall returns -1 on error and sets errno.
    /// The kernel returns -errno directly.
    fn libc_to_kernel(result: i64) -> i64 {
        if result == -1 {
            let errno = std::io::Error::last_os_error().raw_os_error().unwrap_or(1);
            -(errno as i64)
        } else {
            result
        }
    }

    /// Translate m68k open() flags to x86-64 flags.
    /// m68k and x86-64 have different values for O_DIRECTORY, O_NOFOLLOW, O_DIRECT, O_LARGEFILE.
    fn translate_open_flags(m68k_flags: i32) -> i32 {
        // m68k values (from asm/fcntl.h)
        const M68K_O_DIRECTORY: i32 = 0o040000;
        const M68K_O_NOFOLLOW: i32 = 0o100000;
        const M68K_O_DIRECT: i32 = 0o200000;
        const M68K_O_LARGEFILE: i32 = 0o400000;

        // x86-64 values (from asm-generic/fcntl.h)
        const X86_64_O_DIRECT: i32 = 0o040000;
        const X86_64_O_LARGEFILE: i32 = 0o100000;
        const X86_64_O_DIRECTORY: i32 = 0o200000;
        const X86_64_O_NOFOLLOW: i32 = 0o400000;

        // Mask for flags that are the same on both architectures
        const COMMON_FLAGS_MASK: i32 = 0o037777;

        // Start with common flags (O_RDONLY, O_WRONLY, O_RDWR, O_CREAT, O_EXCL, etc.)
        let mut x86_flags = m68k_flags & COMMON_FLAGS_MASK;

        // Translate architecture-specific flags
        if m68k_flags & M68K_O_DIRECTORY != 0 {
            x86_flags |= X86_64_O_DIRECTORY;
        }
        if m68k_flags & M68K_O_NOFOLLOW != 0 {
            x86_flags |= X86_64_O_NOFOLLOW;
        }
        if m68k_flags & M68K_O_DIRECT != 0 {
            x86_flags |= X86_64_O_DIRECT;
        }
        if m68k_flags & M68K_O_LARGEFILE != 0 {
            x86_flags |= X86_64_O_LARGEFILE;
        }

        // O_CLOEXEC is at 0o2000000 on both architectures (keep it)
        x86_flags |= m68k_flags & 0o2000000;

        x86_flags
    }

    /// Generic syscall passthrough for syscalls with no pointer arguments.
    fn sys_passthrough(&self, syscall_num: u32, arg_count: usize) -> i64 {
        let arg = |i: usize| self.data_regs[i + 1] as i64;
        let result = unsafe {
            match arg_count {
                0 => libc::syscall(syscall_num as i64),
                1 => libc::syscall(syscall_num as i64, arg(0)),
                2 => libc::syscall(syscall_num as i64, arg(0), arg(1)),
                3 => libc::syscall(syscall_num as i64, arg(0), arg(1), arg(2)),
                4 => libc::syscall(syscall_num as i64, arg(0), arg(1), arg(2), arg(3)),
                _ => libc::syscall(syscall_num as i64, arg(0), arg(1), arg(2), arg(3), arg(4)),
            }
        };
        Self::libc_to_kernel(result)
    }

    /// Check if an fd is set in a guest fd_set (m68k format: 32-bit big-endian longs)
    fn guest_fd_isset(&self, fd: i32, fdset_addr: usize) -> Result<bool> {
        // fd_set on m68k uses 32-bit longs, so we have 32 longs (128 bytes total)
        // Each long holds 32 bits
        let long_index = (fd / 32) as usize;
        let bit_index = fd % 32;

        // Read the 32-bit big-endian long
        let long_addr = fdset_addr + long_index * 4;
        let long_val = self.memory.read_long(long_addr)?;

        // Check if the bit is set (bit 0 is the LSB)
        Ok((long_val & (1 << bit_index)) != 0)
    }

    /// Convert guest fd_set to host fd_set
    fn guest_to_host_fdset(&self, guest_addr: usize, nfds: i32) -> Result<libc::fd_set> {
        let mut host_set: libc::fd_set = unsafe { std::mem::zeroed() };

        for fd in 0..nfds {
            if self.guest_fd_isset(fd, guest_addr)? {
                unsafe {
                    libc::FD_SET(fd, &mut host_set);
                }
            }
        }

        Ok(host_set)
    }

    /// Copy host fd_set back to guest fd_set
    fn host_to_guest_fdset(
        &mut self,
        host_set: &libc::fd_set,
        guest_addr: usize,
        nfds: i32,
    ) -> Result<()> {
        // Clear the guest fd_set first (32 longs * 4 bytes = 128 bytes)
        for i in 0..32 {
            let zero_bytes = [0u8; 4];
            self.memory.write_data(guest_addr + i * 4, &zero_bytes)?;
        }

        // Set bits for each fd that's set in the host set
        for fd in 0..nfds {
            if unsafe { libc::FD_ISSET(fd, host_set) } {
                let long_index = (fd / 32) as usize;
                let bit_index = fd % 32;
                let long_addr = guest_addr + long_index * 4;

                let current = self.memory.read_long(long_addr)?;
                let new_val = current | (1 << bit_index);

                // Write as big-endian bytes
                let bytes = new_val.to_be_bytes();
                self.memory.write_data(long_addr, &bytes)?;
            }
        }

        Ok(())
    }

    fn build_iovecs(
        &mut self,
        base_addr: usize,
        count: usize,
        writable: bool,
    ) -> Result<Vec<libc::iovec>> {
        let mut iovecs = Vec::with_capacity(count);
        for i in 0..count {
            let entry = base_addr + i * 8;
            let iov_base = self.memory.read_long(entry)? as usize;
            let iov_len = self.memory.read_long(entry + 4)? as usize;
            if iov_len == 0 {
                // Zero-length iovecs are allowed; use null pointer.
                iovecs.push(libc::iovec {
                    iov_base: std::ptr::null_mut(),
                    iov_len,
                });
                continue;
            }

            let host_ptr = if writable {
                self.memory
                    .guest_to_host_mut(iov_base, iov_len)
                    .ok_or_else(|| anyhow!("invalid iovec buffer {iov_base:#x} (len {iov_len})"))?
            } else {
                self.memory
                    .guest_to_host(iov_base, iov_len)
                    .ok_or_else(|| anyhow!("invalid iovec buffer {iov_base:#x} (len {iov_len})"))?
            };

            iovecs.push(libc::iovec {
                iov_base: host_ptr as *mut libc::c_void,
                iov_len,
            });
        }
        Ok(iovecs)
    }

    fn alloc_anonymous_mmap(&mut self, req_addr: usize, length: usize, prot: i32) -> Result<usize> {
        use crate::memory::MemorySegment;
        use goblin::elf::program_header;

        let aligned_len = (length + 4095) & !4095;
        let addr = if req_addr != 0 {
            req_addr
        } else {
            self.memory
                .find_free_range(aligned_len)
                .ok_or_else(|| anyhow!("mmap: no free address range for {aligned_len} bytes"))?
        };

        let mut elf_flags = 0u32;
        if prot & 0x1 != 0 {
            elf_flags |= program_header::PF_R;
        }
        if prot & 0x2 != 0 {
            elf_flags |= program_header::PF_W;
        }
        if prot & 0x4 != 0 {
            elf_flags |= program_header::PF_X;
        }

        self.memory.add_segment(MemorySegment {
            vaddr: addr,
            data: crate::memory::MemoryData::Owned(vec![0u8; aligned_len]),
            flags: elf_flags,
            align: 4096,
        });

        Ok(addr)
    }

    fn read_itimerval(&self, addr: usize) -> Result<libc::itimerval> {
        // m68k uclibc uses 64-bit time_t
        let it_interval_sec_bytes: [u8; 8] = self.memory.read_data(addr, 8)?.try_into().unwrap();
        let it_interval_sec = i64::from_be_bytes(it_interval_sec_bytes) as libc::time_t;
        let it_interval_usec = self.memory.read_long(addr + 8)? as libc::suseconds_t;

        let it_value_sec_bytes: [u8; 8] = self.memory.read_data(addr + 12, 8)?.try_into().unwrap();
        let it_value_sec = i64::from_be_bytes(it_value_sec_bytes) as libc::time_t;
        let it_value_usec = self.memory.read_long(addr + 20)? as libc::suseconds_t;

        Ok(libc::itimerval {
            it_interval: libc::timeval {
                tv_sec: it_interval_sec,
                tv_usec: it_interval_usec,
            },
            it_value: libc::timeval {
                tv_sec: it_value_sec,
                tv_usec: it_value_usec,
            },
        })
    }

    fn write_itimerval(&mut self, addr: usize, val: &libc::itimerval) -> Result<()> {
        // m68k uclibc uses 64-bit time_t
        self.memory
            .write_data(addr, &val.it_interval.tv_sec.to_be_bytes())?;
        self.memory
            .write_data(addr + 8, &(val.it_interval.tv_usec as u32).to_be_bytes())?;
        self.memory
            .write_data(addr + 12, &val.it_value.tv_sec.to_be_bytes())?;
        self.memory
            .write_data(addr + 20, &(val.it_value.tv_usec as u32).to_be_bytes())?;
        Ok(())
    }

    fn write_statx(&mut self, addr: usize, sx: &libc::statx) -> Result<()> {
        // statx structure has the same layout across all architectures
        // See: include/uapi/linux/stat.h
        let mut offset = addr;

        // u32 stx_mask
        self.memory.write_data(offset, &sx.stx_mask.to_be_bytes())?;
        offset += 4;

        // u32 stx_blksize
        self.memory
            .write_data(offset, &sx.stx_blksize.to_be_bytes())?;
        offset += 4;

        // u64 stx_attributes
        self.memory
            .write_data(offset, &sx.stx_attributes.to_be_bytes())?;
        offset += 8;

        // u32 stx_nlink
        self.memory
            .write_data(offset, &sx.stx_nlink.to_be_bytes())?;
        offset += 4;

        // u32 stx_uid
        self.memory.write_data(offset, &sx.stx_uid.to_be_bytes())?;
        offset += 4;

        // u32 stx_gid
        self.memory.write_data(offset, &sx.stx_gid.to_be_bytes())?;
        offset += 4;

        // u16 stx_mode
        self.memory.write_data(offset, &sx.stx_mode.to_be_bytes())?;
        offset += 2;

        // u16 __spare0[1] - padding
        self.memory.write_data(offset, &0u16.to_be_bytes())?;
        offset += 2;

        // u64 stx_ino
        self.memory.write_data(offset, &sx.stx_ino.to_be_bytes())?;
        offset += 8;

        // u64 stx_size
        self.memory.write_data(offset, &sx.stx_size.to_be_bytes())?;
        offset += 8;

        // u64 stx_blocks
        self.memory
            .write_data(offset, &sx.stx_blocks.to_be_bytes())?;
        offset += 8;

        // u64 stx_attributes_mask
        self.memory
            .write_data(offset, &sx.stx_attributes_mask.to_be_bytes())?;
        offset += 8;

        // struct statx_timestamp stx_atime (16 bytes: i64 tv_sec + u32 tv_nsec + i32 __reserved)
        self.memory
            .write_data(offset, &sx.stx_atime.tv_sec.to_be_bytes())?;
        offset += 8;
        self.memory
            .write_data(offset, &sx.stx_atime.tv_nsec.to_be_bytes())?;
        offset += 4;
        self.memory.write_data(offset, &0i32.to_be_bytes())?; // __reserved
        offset += 4;

        // struct statx_timestamp stx_btime (16 bytes)
        self.memory
            .write_data(offset, &sx.stx_btime.tv_sec.to_be_bytes())?;
        offset += 8;
        self.memory
            .write_data(offset, &sx.stx_btime.tv_nsec.to_be_bytes())?;
        offset += 4;
        self.memory.write_data(offset, &0i32.to_be_bytes())?;
        offset += 4;

        // struct statx_timestamp stx_ctime (16 bytes)
        self.memory
            .write_data(offset, &sx.stx_ctime.tv_sec.to_be_bytes())?;
        offset += 8;
        self.memory
            .write_data(offset, &sx.stx_ctime.tv_nsec.to_be_bytes())?;
        offset += 4;
        self.memory.write_data(offset, &0i32.to_be_bytes())?;
        offset += 4;

        // struct statx_timestamp stx_mtime (16 bytes)
        self.memory
            .write_data(offset, &sx.stx_mtime.tv_sec.to_be_bytes())?;
        offset += 8;
        self.memory
            .write_data(offset, &sx.stx_mtime.tv_nsec.to_be_bytes())?;
        offset += 4;
        self.memory.write_data(offset, &0i32.to_be_bytes())?;
        offset += 4;

        // u32 stx_rdev_major
        self.memory
            .write_data(offset, &sx.stx_rdev_major.to_be_bytes())?;
        offset += 4;

        // u32 stx_rdev_minor
        self.memory
            .write_data(offset, &sx.stx_rdev_minor.to_be_bytes())?;
        offset += 4;

        // u32 stx_dev_major
        self.memory
            .write_data(offset, &sx.stx_dev_major.to_be_bytes())?;
        offset += 4;

        // u32 stx_dev_minor
        self.memory
            .write_data(offset, &sx.stx_dev_minor.to_be_bytes())?;
        offset += 4;

        // u64 stx_mnt_id
        self.memory
            .write_data(offset, &sx.stx_mnt_id.to_be_bytes())?;
        offset += 8;

        // u32 stx_dio_mem_align
        #[cfg(target_os = "linux")]
        {
            // This field might not be available on older libc versions, so we write 0
            self.memory.write_data(offset, &0u32.to_be_bytes())?;
        }
        #[cfg(not(target_os = "linux"))]
        {
            self.memory.write_data(offset, &0u32.to_be_bytes())?;
        }
        offset += 4;

        // u32 stx_dio_offset_align
        self.memory.write_data(offset, &0u32.to_be_bytes())?;
        // offset += 4;

        // u64 __spare3[12] - spare fields at the end
        // We can skip these as they're zero

        Ok(())
    }

    fn write_stat(&mut self, addr: usize, s: &libc::stat) -> Result<()> {
        // m68k stat struct layout (32-bit)
        self.memory
            .write_data(addr, &(s.st_dev as u32).to_be_bytes())?;
        self.memory
            .write_data(addr + 4, &(s.st_ino as u32).to_be_bytes())?;
        self.memory
            .write_data(addr + 8, &(s.st_mode).to_be_bytes())?;
        self.memory
            .write_data(addr + 12, &(s.st_nlink as u32).to_be_bytes())?;
        self.memory
            .write_data(addr + 16, &(s.st_uid).to_be_bytes())?;
        self.memory
            .write_data(addr + 20, &(s.st_gid).to_be_bytes())?;
        self.memory
            .write_data(addr + 24, &(s.st_rdev as u32).to_be_bytes())?;
        self.memory
            .write_data(addr + 28, &(s.st_size as u32).to_be_bytes())?;
        self.memory
            .write_data(addr + 32, &(s.st_blksize as u32).to_be_bytes())?;
        self.memory
            .write_data(addr + 36, &(s.st_blocks as u32).to_be_bytes())?;
        self.memory
            .write_data(addr + 40, &(s.st_atime as u32).to_be_bytes())?;
        self.memory
            .write_data(addr + 44, &(s.st_mtime as u32).to_be_bytes())?;
        self.memory
            .write_data(addr + 48, &(s.st_ctime as u32).to_be_bytes())?;
        Ok(())
    }

    /// Read xattr_args structure from guest memory.
    fn read_xattr_args(&self, addr: usize) -> Result<(usize, usize, u32)> {
        let value_ptr = self.memory.read_long(addr)? as usize;
        let size = self.memory.read_long(addr + 4)? as usize;
        let flags = self.memory.read_long(addr + 8)?;
        Ok((value_ptr, size, flags))
    }

    /// listxattr(path, list, size)
    fn sys_listxattr(&mut self) -> Result<i64> {
        let path_ptr = self.data_regs[1] as usize;
        let list_ptr = self.data_regs[2] as usize;
        let size = self.data_regs[3] as usize;

        let path = self.read_c_string(path_ptr)?;
        let buf_host = if list_ptr != 0 && size > 0 {
            self.memory
                .guest_to_host_mut(list_ptr, size)
                .ok_or_else(|| anyhow!("invalid xattr list buffer"))?
        } else {
            std::ptr::null_mut()
        };

        let res = unsafe {
            libc::listxattr(
                path.as_ptr() as *const libc::c_char,
                buf_host as *mut libc::c_char,
                size,
            )
        };
        Ok(Self::libc_to_kernel(res as i64))
    }

    /// llistxattr(path, list, size)
    fn sys_llistxattr(&mut self) -> Result<i64> {
        let path_ptr = self.data_regs[1] as usize;
        let list_ptr = self.data_regs[2] as usize;
        let size = self.data_regs[3] as usize;

        let path = self.read_c_string(path_ptr)?;
        let buf_host = if list_ptr != 0 && size > 0 {
            self.memory
                .guest_to_host_mut(list_ptr, size)
                .ok_or_else(|| anyhow!("invalid xattr list buffer"))?
        } else {
            std::ptr::null_mut()
        };

        let res = unsafe {
            libc::llistxattr(
                path.as_ptr() as *const libc::c_char,
                buf_host as *mut libc::c_char,
                size,
            )
        };
        Ok(Self::libc_to_kernel(res as i64))
    }

    /// flistxattr(fd, list, size)
    fn sys_flistxattr(&mut self) -> Result<i64> {
        let fd = self.data_regs[1] as libc::c_int;
        let list_ptr = self.data_regs[2] as usize;
        let size = self.data_regs[3] as usize;

        let buf_host = if list_ptr != 0 && size > 0 {
            self.memory
                .guest_to_host_mut(list_ptr, size)
                .ok_or_else(|| anyhow!("invalid xattr list buffer"))?
        } else {
            std::ptr::null_mut()
        };

        let res = unsafe { libc::flistxattr(fd, buf_host as *mut libc::c_char, size) };
        Ok(Self::libc_to_kernel(res as i64))
    }

    /// removexattr(path, name)
    fn sys_removexattr(&mut self) -> Result<i64> {
        let path_ptr = self.data_regs[1] as usize;
        let name_ptr = self.data_regs[2] as usize;

        let path = self.read_c_string(path_ptr)?;
        let name = self.read_c_string(name_ptr)?;

        let res = unsafe {
            libc::removexattr(
                path.as_ptr() as *const libc::c_char,
                name.as_ptr() as *const libc::c_char,
            )
        };
        Ok(Self::libc_to_kernel(res as i64))
    }

    /// lremovexattr(path, name)
    fn sys_lremovexattr(&mut self) -> Result<i64> {
        let path_ptr = self.data_regs[1] as usize;
        let name_ptr = self.data_regs[2] as usize;

        let path = self.read_c_string(path_ptr)?;
        let name = self.read_c_string(name_ptr)?;

        let res = unsafe {
            libc::lremovexattr(
                path.as_ptr() as *const libc::c_char,
                name.as_ptr() as *const libc::c_char,
            )
        };
        Ok(Self::libc_to_kernel(res as i64))
    }

    /// fremovexattr(fd, name)
    fn sys_fremovexattr(&mut self) -> Result<i64> {
        let fd = self.data_regs[1] as libc::c_int;
        let name_ptr = self.data_regs[2] as usize;

        let name = self.read_c_string(name_ptr)?;

        let res = unsafe { libc::fremovexattr(fd, name.as_ptr() as *const libc::c_char) };
        Ok(Self::libc_to_kernel(res as i64))
    }

    /// setxattrat(dirfd, path, name, value, size, flags)
    fn sys_setxattrat(&mut self) -> Result<i64> {
        let dirfd = self.data_regs[1] as libc::c_int;
        let path_ptr = self.data_regs[2] as usize;
        let name_ptr = self.data_regs[3] as usize;
        let value_ptr = self.data_regs[4] as usize;
        let size = self.data_regs[5] as usize;
        let flags = self.data_regs[6] as i32;

        let path = self.read_c_string(path_ptr)?;
        let name = self.read_c_string(name_ptr)?;
        let value = if value_ptr != 0 && size > 0 {
            let host = self
                .memory
                .guest_to_host(value_ptr, size)
                .ok_or_else(|| anyhow!("invalid xattr value buffer"))?;
            Some(unsafe { std::slice::from_raw_parts(host, size) })
        } else {
            None
        };

        let res = unsafe {
            libc::syscall(
                463, // setxattrat syscall number on x86_64 as well
                dirfd,
                path.as_ptr(),
                name.as_ptr(),
                value
                    .map(|v| v.as_ptr() as *const libc::c_void)
                    .unwrap_or(std::ptr::null()),
                size,
                flags,
                0, // at_flags - always 0 for now
            )
        };
        Ok(Self::libc_to_kernel(res as i64))
    }

    /// getxattrat(dirfd, path, name, value, size)
    fn sys_getxattrat(&mut self) -> Result<i64> {
        let dirfd = self.data_regs[1] as libc::c_int;
        let path_ptr = self.data_regs[2] as usize;
        let name_ptr = self.data_regs[3] as usize;
        let value_ptr = self.data_regs[4] as usize;
        let size = self.data_regs[5] as usize;

        let path = self.read_c_string(path_ptr)?;
        let name = self.read_c_string(name_ptr)?;
        let buf_host = if value_ptr != 0 && size > 0 {
            self.memory
                .guest_to_host_mut(value_ptr, size)
                .ok_or_else(|| anyhow!("invalid xattr buffer"))?
        } else {
            std::ptr::null_mut()
        };

        let res = unsafe {
            libc::syscall(
                464, // getxattrat syscall number on x86_64 as well
                dirfd,
                path.as_ptr(),
                name.as_ptr(),
                buf_host,
                size,
                0, // at_flags - always 0 for now
            )
        };
        Ok(Self::libc_to_kernel(res as i64))
    }

    /// listxattrat(dirfd, path, args, atflags)
    fn sys_listxattrat(&mut self) -> Result<i64> {
        let dirfd = self.data_regs[1] as libc::c_int;
        let path_ptr = self.data_regs[2] as usize;
        let args_ptr = self.data_regs[3] as usize;
        let atflags = self.data_regs[4] as libc::c_int;

        let (value_ptr, size, _flags) = self.read_xattr_args(args_ptr)?;
        let path = self.read_c_string(path_ptr)?;
        let buf_host = if value_ptr != 0 && size > 0 {
            self.memory
                .guest_to_host_mut(value_ptr, size)
                .ok_or_else(|| anyhow!("invalid xattr buffer"))?
        } else {
            std::ptr::null_mut()
        };

        let res = unsafe {
            libc::syscall(
                465, // listxattrat syscall number on x86_64 as well
                dirfd,
                path.as_ptr(),
                buf_host,
                size,
                atflags,
            )
        };
        Ok(Self::libc_to_kernel(res as i64))
    }

    /// removexattrat(dirfd, path, name, atflags)
    fn sys_removexattrat(&mut self) -> Result<i64> {
        let dirfd = self.data_regs[1] as libc::c_int;
        let path_ptr = self.data_regs[2] as usize;
        let name_ptr = self.data_regs[3] as usize;
        let atflags = self.data_regs[4] as libc::c_int;

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

    /// landlock_create_ruleset(attr, size, flags)
    /// Creates a new Landlock ruleset and returns a file descriptor
    fn sys_landlock_create_ruleset(&mut self) -> Result<i64> {
        let attr_addr = self.data_regs[1] as usize;
        let size = self.data_regs[2] as usize;
        let flags = self.data_regs[3];

        if attr_addr == 0 || size == 0 {
            let result = unsafe { libc::syscall(444, std::ptr::null::<u8>(), size, flags) };
            return Ok(Self::libc_to_kernel(result as i64));
        }

        // We only need the first 16 bytes (handled_access_fs + handled_access_net).
        // Validate the guest pointer for that range.
        let copy_len = size.min(16);
        self.memory
            .guest_to_host(attr_addr, copy_len)
            .ok_or_else(|| anyhow!("invalid landlock_ruleset_attr"))?;

        // Read and translate the structure from guest (big-endian) to host (little-endian)
        let handled_access_fs = if size >= 8 {
            self.read_u64_be(attr_addr)?
        } else {
            0
        };

        let handled_access_net = if size >= 16 {
            self.read_u64_be(attr_addr + 8)?
        } else {
            0
        };

        // Build a host-endian buffer matching the requested size.
        let mut host_attr = vec![0u8; size];
        let mut fields = [0u8; 16];
        fields[..8].copy_from_slice(&handled_access_fs.to_ne_bytes());
        if size >= 16 {
            fields[8..16].copy_from_slice(&handled_access_net.to_ne_bytes());
        }
        host_attr[..copy_len].copy_from_slice(&fields[..copy_len]);

        let result = unsafe { libc::syscall(444, host_attr.as_ptr(), size, flags) };

        Ok(Self::libc_to_kernel(result as i64))
    }

    /// landlock_add_rule(ruleset_fd, rule_type, rule_attr, flags)
    /// Adds a rule to a Landlock ruleset
    fn sys_landlock_add_rule(&mut self) -> Result<i64> {
        let ruleset_fd = self.data_regs[1] as i32;
        let rule_type = self.data_regs[2];
        let rule_attr_addr = self.data_regs[3] as usize;
        let flags = self.data_regs[4];

        // Rule type determines the structure size
        // LANDLOCK_RULE_PATH_BENEATH = 1: struct landlock_path_beneath_attr (16 bytes)
        // LANDLOCK_RULE_NET_PORT = 2: struct landlock_net_port_attr (16 bytes)
        const LANDLOCK_RULE_PATH_BENEATH: u32 = 1;
        const LANDLOCK_RULE_NET_PORT: u32 = 2;

        if rule_attr_addr == 0 {
            let result =
                unsafe { libc::syscall(445, ruleset_fd, rule_type, std::ptr::null::<u8>(), flags) };
            return Ok(Self::libc_to_kernel(result as i64));
        }

        let result = match rule_type {
            LANDLOCK_RULE_PATH_BENEATH => {
                // struct landlock_path_beneath_attr { u64 allowed_access; i32 parent_fd; }
                self.memory
                    .guest_to_host(rule_attr_addr, 12)
                    .ok_or_else(|| anyhow!("invalid landlock_path_beneath_attr"))?;

                let allowed_access = self.read_u64_be(rule_attr_addr)?;
                let parent_fd = self.memory.read_long(rule_attr_addr + 8)? as i32;

                let mut host_attr = [0u8; 16];
                host_attr[..8].copy_from_slice(&allowed_access.to_ne_bytes());
                host_attr[8..12].copy_from_slice(&parent_fd.to_ne_bytes());

                unsafe { libc::syscall(445, ruleset_fd, rule_type, host_attr.as_ptr(), flags) }
            }
            LANDLOCK_RULE_NET_PORT => {
                // struct landlock_net_port_attr { u64 allowed_access; u64 port; }
                self.memory
                    .guest_to_host(rule_attr_addr, 16)
                    .ok_or_else(|| anyhow!("invalid landlock_net_port_attr"))?;

                let allowed_access = self.read_u64_be(rule_attr_addr)?;
                let port = self.read_u64_be(rule_attr_addr + 8)?;

                let mut host_attr = [0u8; 16];
                host_attr[..8].copy_from_slice(&allowed_access.to_ne_bytes());
                host_attr[8..16].copy_from_slice(&port.to_ne_bytes());

                unsafe { libc::syscall(445, ruleset_fd, rule_type, host_attr.as_ptr(), flags) }
            }
            _ => return Ok(Self::libc_to_kernel(-libc::EINVAL as i64)),
        };

        Ok(Self::libc_to_kernel(result as i64))
    }

    /// landlock_restrict_self(ruleset_fd, flags)
    /// Enforces the ruleset on the calling thread
    fn sys_landlock_restrict_self(&mut self) -> Result<i64> {
        let ruleset_fd = self.data_regs[1] as i32;
        let flags = self.data_regs[2];

        // Simple passthrough - no structure translation needed
        let result = unsafe { libc::syscall(446, ruleset_fd, flags) };

        Ok(Self::libc_to_kernel(result as i64))
    }

    /// sysinfo(info)
    fn sys_sysinfo(&mut self) -> Result<i64> {
        let info_addr = self.data_regs[1] as usize;
        let mut info: libc::sysinfo = unsafe { std::mem::zeroed() };
        let result = unsafe { libc::sysinfo(&mut info) };
        if result == 0 {
            self.memory
                .write_data(info_addr, &(info.uptime as u32).to_be_bytes())?;
            self.memory
                .write_data(info_addr + 4, &(info.loads[0] as u32).to_be_bytes())?;
            self.memory
                .write_data(info_addr + 8, &(info.loads[1] as u32).to_be_bytes())?;
            self.memory
                .write_data(info_addr + 12, &(info.loads[2] as u32).to_be_bytes())?;
            self.memory
                .write_data(info_addr + 16, &(info.totalram as u32).to_be_bytes())?;
            self.memory
                .write_data(info_addr + 20, &(info.freeram as u32).to_be_bytes())?;
            self.memory
                .write_data(info_addr + 24, &(info.sharedram as u32).to_be_bytes())?;
            self.memory
                .write_data(info_addr + 28, &(info.bufferram as u32).to_be_bytes())?;
            self.memory
                .write_data(info_addr + 32, &(info.totalswap as u32).to_be_bytes())?;
            self.memory
                .write_data(info_addr + 36, &(info.freeswap as u32).to_be_bytes())?;
            self.memory
                .write_data(info_addr + 40, &(info.procs as u16).to_be_bytes())?;
        }
        Ok(result as i64)
    }

    /// uname(buf)
    fn sys_uname(&mut self) -> Result<i64> {
        let buf_addr = self.data_regs[1] as usize;
        let mut uts: libc::utsname = unsafe { std::mem::zeroed() };
        let result = unsafe { libc::uname(&mut uts) };
        if result == 0 {
            // Each field is 65 bytes in the kernel struct
            let field_size = 65usize;
            self.memory.write_data(
                buf_addr,
                &uts.sysname[..field_size]
                    .iter()
                    .map(|&c| c as u8)
                    .collect::<Vec<_>>(),
            )?;
            self.memory.write_data(
                buf_addr + field_size,
                &uts.nodename[..field_size]
                    .iter()
                    .map(|&c| c as u8)
                    .collect::<Vec<_>>(),
            )?;
            self.memory.write_data(
                buf_addr + field_size * 2,
                &uts.release[..field_size]
                    .iter()
                    .map(|&c| c as u8)
                    .collect::<Vec<_>>(),
            )?;
            self.memory.write_data(
                buf_addr + field_size * 3,
                &uts.version[..field_size]
                    .iter()
                    .map(|&c| c as u8)
                    .collect::<Vec<_>>(),
            )?;
            self.memory.write_data(
                buf_addr + field_size * 4,
                &uts.machine[..field_size]
                    .iter()
                    .map(|&c| c as u8)
                    .collect::<Vec<_>>(),
            )?;
        }
        Ok(result as i64)
    }

    /// _llseek(fd, offset_high, offset_low, result, whence)
    fn sys_llseek(&mut self) -> Result<i64> {
        let fd = self.data_regs[1] as i32;
        let offset_high = self.data_regs[2];
        let offset_low = self.data_regs[3];
        let result_addr = self.data_regs[4] as usize;
        let whence = self.data_regs[5] as i32;

        let offset = ((offset_high as i64) << 32) | (offset_low as i64);
        let result = unsafe { libc::lseek(fd, offset, whence) };

        if result >= 0 && result_addr != 0 {
            // Write 64-bit result to guest memory
            self.memory
                .write_data(result_addr, &((result >> 32) as u32).to_be_bytes())?;
            self.memory
                .write_data(result_addr + 4, &(result as u32).to_be_bytes())?;
            Ok(0)
        } else {
            Ok(result)
        }
    }

    /// getdents(fd, dirp, count) - 32-bit dirent
    fn sys_getdents32(&mut self) -> Result<i64> {
        let fd = self.data_regs[1] as i32;
        let dirp = self.data_regs[2] as usize;
        let count = self.data_regs[3] as usize;

        // Read into a temporary host buffer
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

        // eprintln!("getdents32: fd={}, count={}, bytes_read={}", fd, count, bytes_read);

        // Translate dirent64 structures to 32-bit m68k dirent format
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

            // Find null terminator in d_name
            let name_start = host_off + 19;
            let name_end = host_buf[name_start..host_off + d_reclen as usize]
                .iter()
                .position(|&b| b == 0)
                .map(|p| name_start + p)
                .unwrap_or(host_off + d_reclen as usize);

            let name_len = name_end - name_start;

            // m68k linux_dirent structure (OLD format, not linux_dirent64):
            // u32 d_ino (4 bytes, BE)
            // i32 d_off (4 bytes, BE)
            // u16 d_reclen (2 bytes, BE)
            // char d_name[] (variable, null-terminated)
            // [padding to align]
            // u8  d_type (1 byte, at offset reclen-1)

            // Calculate m68k record length (aligned to 2 bytes, includes d_type at end)
            let m68k_reclen = (10 + name_len + 1 + 1).div_ceil(2) * 2;

            if guest_off + m68k_reclen > count {
                break;
            }

            // Write m68k linux_dirent (truncate 64-bit values to 32-bit)
            self.memory
                .write_data(dirp + guest_off, &(d_ino as u32).to_be_bytes())?;
            self.memory
                .write_data(dirp + guest_off + 4, &(d_off as i32).to_be_bytes())?;
            self.memory
                .write_data(dirp + guest_off + 8, &(m68k_reclen as u16).to_be_bytes())?;

            // Write name at offset 10
            self.memory.write_data(
                dirp + guest_off + 10,
                &host_buf[name_start..name_start + name_len],
            )?;
            self.memory
                .write_data(dirp + guest_off + 10 + name_len, &[0u8])?;

            // Zero out padding
            for i in (10 + name_len + 1)..(m68k_reclen - 1) {
                self.memory.write_data(dirp + guest_off + i, &[0u8])?;
            }

            // Write d_type at the LAST byte of the record (reclen - 1)
            self.memory
                .write_data(dirp + guest_off + m68k_reclen - 1, &[d_type])?;

            host_off += d_reclen as usize;
            guest_off += m68k_reclen;
        }
        Ok(guest_off as i64)
    }

    /// getdents64(fd, dirp, count) - 64-bit dirent64
    fn sys_getdents64(&mut self) -> Result<i64> {
        let fd = self.data_regs[1] as i32;
        let dirp = self.data_regs[2] as usize;
        let count = self.data_regs[3] as usize;

        // Read into a temporary host buffer
        let mut host_buf = vec![0u8; count];
        let result =
            unsafe { libc::syscall(libc::SYS_getdents64, fd, host_buf.as_mut_ptr(), count) };

        if result < 0 {
            return Ok(Self::libc_to_kernel(result));
        }

        let bytes_read = result as usize;

        if bytes_read == 0 {
            // End of directory
            return Ok(0);
        }

        // Translate dirent64 structures from x86-64 to m68k format
        let mut host_off = 0;
        let mut guest_off = 0;

        while host_off < bytes_read {
            // Read x86-64 linux_dirent64:
            // struct linux_dirent64 {
            //     u64 d_ino;
            //     i64 d_off;
            //     u16 d_reclen;
            //     u8  d_type;
            //     char d_name[];
            // }

            if host_off + 19 > bytes_read {
                break; // Not enough data for header
            }

            let d_ino = u64::from_ne_bytes(host_buf[host_off..host_off + 8].try_into()?);
            let d_off = i64::from_ne_bytes(host_buf[host_off + 8..host_off + 16].try_into()?);
            let d_reclen = u16::from_ne_bytes(host_buf[host_off + 16..host_off + 18].try_into()?);
            let d_type = host_buf[host_off + 18];

            // Find null terminator in d_name
            let name_start = host_off + 19;
            let name_end = host_buf[name_start..host_off + d_reclen as usize]
                .iter()
                .position(|&b| b == 0)
                .map(|p| name_start + p)
                .unwrap_or(host_off + d_reclen as usize);

            let name_len = name_end - name_start;

            // m68k dirent64 structure (same layout, but ensure big-endian):
            // u64 d_ino (8 bytes, BE)
            // i64 d_off (8 bytes, BE)
            // u16 d_reclen (2 bytes, BE)
            // u8  d_type (1 byte)
            // char d_name[] (variable, null-terminated)

            // Calculate m68k record length (aligned to 8 bytes)
            let m68k_reclen = (19 + name_len + 1).div_ceil(8) * 8;

            if guest_off + m68k_reclen > count {
                break; // Not enough space in guest buffer
            }

            // Write m68k dirent64
            self.memory
                .write_data(dirp + guest_off, &d_ino.to_be_bytes())?;
            self.memory
                .write_data(dirp + guest_off + 8, &d_off.to_be_bytes())?;
            self.memory
                .write_data(dirp + guest_off + 16, &(m68k_reclen as u16).to_be_bytes())?;
            self.memory.write_data(dirp + guest_off + 18, &[d_type])?;

            // Write name
            self.memory.write_data(
                dirp + guest_off + 19,
                &host_buf[name_start..name_start + name_len],
            )?;
            self.memory
                .write_data(dirp + guest_off + 19 + name_len, &[0u8])?; // null terminator

            // Zero out padding
            for i in (19 + name_len + 1)..m68k_reclen {
                self.memory.write_data(dirp + guest_off + i, &[0u8])?;
            }

            host_off += d_reclen as usize;
            guest_off += m68k_reclen;
        }

        Ok(guest_off as i64)
    }

    /// sched_setparam(pid, param)
    /// struct sched_param { int sched_priority; }
    fn sys_sched_setparam(&mut self) -> Result<i64> {
        let pid = self.data_regs[1] as libc::pid_t;
        let param_addr = self.data_regs[2] as usize;

        // Read m68k sched_param (4 bytes - int32)
        let priority = self.memory.read_long(param_addr)? as i32;

        // Create host sched_param
        let param = libc::sched_param {
            sched_priority: priority,
        };

        let result = unsafe { libc::sched_setparam(pid, &param) };
        Ok(Self::libc_to_kernel(result as i64))
    }

    /// sched_getparam(pid, param)
    fn sys_sched_getparam(&mut self) -> Result<i64> {
        let pid = self.data_regs[1] as libc::pid_t;
        let param_addr = self.data_regs[2] as usize;

        let mut param: libc::sched_param = unsafe { std::mem::zeroed() };

        let result = unsafe { libc::sched_getparam(pid, &mut param) };
        if result == 0 {
            // Write back sched_priority (4 bytes)
            self.memory
                .write_data(param_addr, &(param.sched_priority as u32).to_be_bytes())?;
        }
        Ok(Self::libc_to_kernel(result as i64))
    }

    /// sched_setscheduler(pid, policy, param)
    fn sys_sched_setscheduler(&mut self) -> Result<i64> {
        let pid = self.data_regs[1] as libc::pid_t;
        let policy = self.data_regs[2] as i32;
        let param_addr = self.data_regs[3] as usize;

        // Read m68k sched_param
        let priority = self.memory.read_long(param_addr)? as i32;

        let param = libc::sched_param {
            sched_priority: priority,
        };

        let result = unsafe { libc::sched_setscheduler(pid, policy, &param) };
        Ok(Self::libc_to_kernel(result as i64))
    }

    /// sched_rr_get_interval(pid, tp)
    fn sys_sched_rr_get_interval(&mut self) -> Result<i64> {
        let pid = self.data_regs[1] as libc::pid_t;
        let tp_addr = self.data_regs[2] as usize;

        let mut tp: libc::timespec = unsafe { std::mem::zeroed() };

        let result = unsafe { libc::sched_rr_get_interval(pid, &mut tp) };
        if result == 0 {
            // m68k uclibc uses 64-bit time_t
            // Write tv_sec as 8 bytes
            self.memory
                .write_data(tp_addr, &(tp.tv_sec as i64).to_be_bytes())?;
            // Write tv_nsec as 4 bytes
            self.memory
                .write_data(tp_addr + 8, &(tp.tv_nsec as u32).to_be_bytes())?;
        }
        Ok(Self::libc_to_kernel(result as i64))
    }

    /// poll(fds, nfds, timeout)
    fn sys_poll(&mut self) -> Result<i64> {
        let fds_addr = self.data_regs[1] as usize;
        let nfds = self.data_regs[2] as usize;
        let timeout = self.data_regs[3] as i32;

        // Read pollfd array from guest (each pollfd is 8 bytes on m68k)
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

        // Write back revents
        for (i, pfd) in pollfds.iter().enumerate() {
            self.memory
                .write_data(fds_addr + i * 8 + 6, &(pfd.revents as u16).to_be_bytes())?;
        }
        Ok(result as i64)
    }

    /// pread64(fd, buf, count, offset)
    /// splice(fd_in, off_in, fd_out, off_out, len, flags)
    fn sys_splice(&mut self) -> Result<i64> {
        let fd_in = self.data_regs[1] as i32;
        let off_in_addr = self.data_regs[2] as usize;
        let fd_out = self.data_regs[3] as i32;
        let off_out_addr = self.data_regs[4] as usize;
        let len = self.data_regs[5] as usize;
        let flags = self.data_regs[6];

        // Read offsets if provided (loff_t is i64, need to read 8 bytes)
        let mut off_in_val = if off_in_addr != 0 {
            let high = self.memory.read_long(off_in_addr)?;
            let low = self.memory.read_long(off_in_addr + 4)?;
            ((high as i64) << 32) | (low as i64)
        } else {
            0
        };

        let mut off_out_val = if off_out_addr != 0 {
            let high = self.memory.read_long(off_out_addr)?;
            let low = self.memory.read_long(off_out_addr + 4)?;
            ((high as i64) << 32) | (low as i64)
        } else {
            0
        };

        // Prepare pointers
        let off_in_ptr = if off_in_addr != 0 {
            &mut off_in_val as *mut i64
        } else {
            std::ptr::null_mut()
        };
        let off_out_ptr = if off_out_addr != 0 {
            &mut off_out_val as *mut i64
        } else {
            std::ptr::null_mut()
        };

        // Call splice
        let result = unsafe { libc::splice(fd_in, off_in_ptr, fd_out, off_out_ptr, len, flags) };

        // Write back offsets if successful
        if result >= 0 {
            if off_in_addr != 0 {
                let high = (off_in_val >> 32) as u32;
                let low = off_in_val as u32;
                self.memory.write_data(off_in_addr, &high.to_be_bytes())?;
                self.memory
                    .write_data(off_in_addr + 4, &low.to_be_bytes())?;
            }
            if off_out_addr != 0 {
                let high = (off_out_val >> 32) as u32;
                let low = off_out_val as u32;
                self.memory.write_data(off_out_addr, &high.to_be_bytes())?;
                self.memory
                    .write_data(off_out_addr + 4, &low.to_be_bytes())?;
            }
        }

        Ok(result as i64)
    }

    /// vmsplice(fd, iov, nr_segs, flags)
    fn sys_vmsplice(&mut self) -> Result<i64> {
        let fd = self.data_regs[1] as i32;
        let iov_addr = self.data_regs[2] as usize;
        let nr_segs = self.data_regs[3] as usize;
        let flags = self.data_regs[4];

        // Build iovec array (read-only for vmsplice)
        let iovecs = self.build_iovecs(iov_addr, nr_segs, false)?;

        // Call vmsplice
        let result = unsafe { libc::vmsplice(fd, iovecs.as_ptr(), iovecs.len(), flags) };

        Ok(result as i64)
    }

    /// socketpair(domain, type, protocol, sv)
    fn sys_socketpair(&mut self) -> Result<i64> {
        let domain = self.data_regs[1] as i32;
        let socktype = self.data_regs[2] as i32;
        let protocol = self.data_regs[3] as i32;
        let sv_addr = self.data_regs[4] as usize;

        let mut sv: [i32; 2] = [0; 2];
        let result = unsafe { libc::socketpair(domain, socktype, protocol, sv.as_mut_ptr()) };
        if result == 0 {
            self.memory
                .write_data(sv_addr, &(sv[0] as u32).to_be_bytes())?;
            self.memory
                .write_data(sv_addr + 4, &(sv[1] as u32).to_be_bytes())?;
        }
        Ok(result as i64)
    }

    /// bind/connect(sockfd, addr, addrlen) - addr is pointer
    fn sys_socket_addr(&self, syscall_num: u32) -> Result<i64> {
        let (sockfd, addr_ptr, addrlen): (i32, usize, usize) = self.get_args();

        let host_addr = self
            .memory
            .guest_to_host(addr_ptr, addrlen)
            .ok_or_else(|| anyhow!("invalid sockaddr"))?;
        Ok(unsafe { libc::syscall(syscall_num as i64, sockfd, host_addr, addrlen) })
    }

    /// accept4(sockfd, addr, addrlen, flags)
    fn sys_accept4(&mut self) -> Result<i64> {
        let (sockfd, addr_ptr, addrlen_ptr, flags): (i32, usize, usize, i32) = self.get_args();

        if addr_ptr == 0 {
            return Ok(unsafe {
                libc::syscall(
                    libc::SYS_accept4,
                    sockfd,
                    std::ptr::null::<u8>(),
                    std::ptr::null::<u32>(),
                    flags,
                )
            });
        }

        let mut addrlen = self.memory.read_long(addrlen_ptr)?;
        let host_addr = self
            .memory
            .guest_to_host_mut(addr_ptr, addrlen as usize)
            .ok_or_else(|| anyhow!("invalid sockaddr buffer"))?;

        let result = unsafe {
            libc::syscall(
                libc::SYS_accept4,
                sockfd,
                host_addr,
                &mut addrlen as *mut u32,
                flags,
            )
        };
        if result >= 0 {
            self.memory
                .write_data(addrlen_ptr, &addrlen.to_be_bytes())?;
        }
        Ok(result)
    }

    /// getsockopt(sockfd, level, optname, optval, optlen)
    fn sys_getsockopt(&mut self) -> Result<i64> {
        let sockfd = self.data_regs[1] as i32;
        let level = self.data_regs[2] as i32;
        let optname = self.data_regs[3] as i32;
        let optval_ptr = self.data_regs[4] as usize;
        let optlen_ptr = self.data_regs[5] as usize;

        let mut optlen = self.memory.read_long(optlen_ptr)? as libc::socklen_t;
        let host_optval = self
            .memory
            .guest_to_host_mut(optval_ptr, optlen as usize)
            .ok_or_else(|| anyhow!("invalid optval buffer"))?;

        let result = unsafe {
            libc::getsockopt(
                sockfd,
                level,
                optname,
                host_optval as *mut libc::c_void,
                &mut optlen,
            )
        };
        if result == 0 {
            self.memory
                .write_data(optlen_ptr, &(optlen as u32).to_be_bytes())?;
        }
        Ok(result as i64)
    }

    /// setsockopt(sockfd, level, optname, optval, optlen)
    fn sys_setsockopt(&self) -> Result<i64> {
        let sockfd = self.data_regs[1] as i32;
        let level = self.data_regs[2] as i32;
        let optname = self.data_regs[3] as i32;
        let optval_ptr = self.data_regs[4] as usize;
        let optlen = self.data_regs[5] as libc::socklen_t;

        let host_optval = self
            .memory
            .guest_to_host(optval_ptr, optlen as usize)
            .ok_or_else(|| anyhow!("invalid optval buffer"))?;

        Ok(unsafe {
            libc::setsockopt(
                sockfd,
                level,
                optname,
                host_optval as *const libc::c_void,
                optlen,
            ) as i64
        })
    }

    /// getsockname/getpeername(sockfd, addr, addrlen)
    fn sys_getsockname(&mut self) -> Result<i64> {
        let sockfd = self.data_regs[1] as i32;
        let addr_ptr = self.data_regs[2] as usize;
        let addrlen_ptr = self.data_regs[3] as usize;

        let mut addrlen = self.memory.read_long(addrlen_ptr)? as libc::socklen_t;
        let host_addr = self
            .memory
            .guest_to_host_mut(addr_ptr, addrlen as usize)
            .ok_or_else(|| anyhow!("invalid sockaddr buffer"))?;

        // Use getsockname - caller distinguishes via syscall number
        let result =
            unsafe { libc::getsockname(sockfd, host_addr as *mut libc::sockaddr, &mut addrlen) };
        if result == 0 {
            self.memory
                .write_data(addrlen_ptr, &(addrlen as u32).to_be_bytes())?;
        }
        Ok(result as i64)
    }

    /// sendto(sockfd, buf, len, flags, dest_addr, addrlen)
    fn sys_sendto(&self) -> Result<i64> {
        let dest_addr = self.data_regs[5] as usize;
        let (sockfd, buf_ptr, len, flags, _, addrlen): (
            i32,
            usize,
            usize,
            i32,
            usize,
            libc::socklen_t,
        ) = self.get_args();

        let host_buf = self
            .memory
            .guest_to_host(buf_ptr, len)
            .ok_or_else(|| anyhow!("invalid sendto buffer"))?;

        let host_addr = if dest_addr != 0 {
            self.memory
                .guest_to_host(dest_addr, addrlen as usize)
                .ok_or_else(|| anyhow!("error translating sockaddr"))?
        } else {
            std::ptr::null()
        };

        Ok(unsafe {
            libc::sendto(
                sockfd,
                host_buf as *const libc::c_void,
                len,
                flags,
                host_addr as *const libc::sockaddr,
                0,
            ) as i64
        })
    }

    /// recvfrom(sockfd, buf, len, flags, src_addr, addrlen)
    fn sys_recvfrom(&mut self) -> Result<i64> {
        let (sockfd, buf_ptr, len, flags, src_addr): (i32, usize, usize, i32, usize) =
            self.get_args();

        let host_buf = self
            .memory
            .guest_to_host_mut(buf_ptr, len)
            .ok_or_else(|| anyhow!("invalid recvfrom buffer"))?;
        let (host_addr, addrlen) = if src_addr != 0 {
            // For simplicity, use fixed buffer size
            let addrlen: libc::socklen_t = 128;
            let host_addr = self
                .memory
                .guest_to_host_mut(src_addr, addrlen as usize)
                .ok_or_else(|| anyhow!("invalid src_addr buffer"))?;
            (addrlen, host_addr)
        } else {
            (0, std::ptr::null_mut())
        };

        Ok(unsafe {
            libc::recvfrom(
                sockfd,
                host_buf as *mut libc::c_void,
                len,
                flags,
                host_addr as *mut libc::sockaddr,
                addrlen as *mut u32,
            ) as i64
        })
    }

    /// sendmsg(sockfd, msg, flags)
    /// Sends a message on a socket using a msghdr structure
    fn sys_sendmsg(&mut self) -> Result<i64> {
        let (sockfd, msg_addr, flags): (i32, usize, i32) = self.get_args();

        // Read m68k msghdr structure (28 bytes)
        // struct msghdr {
        //     void *msg_name;           // 0: u32
        //     socklen_t msg_namelen;    // 4: u32
        //     struct iovec *msg_iov;    // 8: u32
        //     size_t msg_iovlen;        // 12: u32
        //     void *msg_control;        // 16: u32
        //     size_t msg_controllen;    // 20: u32
        //     int msg_flags;            // 24: i32
        // }
        let msg_name = self.memory.read_long(msg_addr)? as usize;
        let msg_namelen = self.memory.read_long(msg_addr + 4)?;
        let msg_iov = self.memory.read_long(msg_addr + 8)? as usize;
        let msg_iovlen = self.memory.read_long(msg_addr + 12)? as usize;
        let msg_control = self.memory.read_long(msg_addr + 16)? as usize;
        let msg_controllen = self.memory.read_long(msg_addr + 20)? as usize;

        // Build iovec array
        let iovecs = if msg_iovlen > 0 {
            self.build_iovecs(msg_iov, msg_iovlen, false)?
        } else {
            Vec::new()
        };

        // Translate msg_name pointer
        let name_ptr = if msg_name != 0 && msg_namelen > 0 {
            self.memory
                .guest_to_host(msg_name, msg_namelen as usize)
                .ok_or_else(|| anyhow!("invalid msg_name pointer"))?
                as *const libc::c_void
        } else {
            std::ptr::null()
        };

        // Translate msg_control pointer
        let control_ptr = if msg_control != 0 && msg_controllen > 0 {
            self.memory
                .guest_to_host(msg_control, msg_controllen)
                .ok_or_else(|| anyhow!("invalid msg_control pointer"))?
                as *const libc::c_void
        } else {
            std::ptr::null()
        };

        // Build host msghdr
        let host_msg = libc::msghdr {
            msg_name: name_ptr as *mut libc::c_void,
            msg_namelen,
            msg_iov: iovecs.as_ptr() as *mut libc::iovec,
            msg_iovlen: iovecs.len(),
            msg_control: control_ptr as *mut libc::c_void,
            msg_controllen,
            msg_flags: 0, // Input flags ignored on sendmsg
        };

        let result = unsafe { libc::sendmsg(sockfd, &host_msg, flags) };
        Ok(result as i64)
    }

    /// recvmsg(sockfd, msg, flags)
    /// Receives a message from a socket using a msghdr structure
    fn sys_recvmsg(&mut self) -> Result<i64> {
        let (sockfd, msg_addr, flags): (i32, usize, i32) = self.get_args();

        // Read m68k msghdr structure (28 bytes)
        let msg_name = self.memory.read_long(msg_addr)? as usize;
        let msg_namelen = self.memory.read_long(msg_addr + 4)?;
        let msg_iov = self.memory.read_long(msg_addr + 8)? as usize;
        let msg_iovlen = self.memory.read_long(msg_addr + 12)? as usize;
        let msg_control = self.memory.read_long(msg_addr + 16)? as usize;
        let msg_controllen = self.memory.read_long(msg_addr + 20)? as usize;

        // Build iovec array (writable for recvmsg)
        let iovecs = if msg_iovlen > 0 {
            self.build_iovecs(msg_iov, msg_iovlen, true)?
        } else {
            Vec::new()
        };

        // Translate msg_name pointer (writable for recvmsg)
        let name_ptr = if msg_name != 0 && msg_namelen > 0 {
            self.memory
                .guest_to_host_mut(msg_name, msg_namelen as usize)
                .ok_or_else(|| anyhow!("invalid msg_name pointer"))?
                as *mut libc::c_void
        } else {
            std::ptr::null_mut()
        };

        // Translate msg_control pointer (writable for recvmsg)
        let control_ptr = if msg_control != 0 && msg_controllen > 0 {
            self.memory
                .guest_to_host_mut(msg_control, msg_controllen)
                .ok_or_else(|| anyhow!("invalid msg_control pointer"))?
                as *mut libc::c_void
        } else {
            std::ptr::null_mut()
        };

        // Build host msghdr
        let mut host_msg = libc::msghdr {
            msg_name: name_ptr,
            msg_namelen,
            msg_iov: iovecs.as_ptr() as *mut libc::iovec,
            msg_iovlen: iovecs.len(),
            msg_control: control_ptr,
            msg_controllen,
            msg_flags: 0,
        };

        let result = unsafe { libc::recvmsg(sockfd, &mut host_msg, flags) };

        // Write back updated fields
        if result >= 0 {
            // msg_namelen may be updated by kernel
            self.memory
                .write_data(msg_addr + 4, &(host_msg.msg_namelen as u32).to_be_bytes())?;
            // msg_controllen may be updated by kernel
            self.memory.write_data(
                msg_addr + 20,
                &(host_msg.msg_controllen as u32).to_be_bytes(),
            )?;
            // msg_flags contains received flags
            self.memory
                .write_data(msg_addr + 24, &(host_msg.msg_flags as i32).to_be_bytes())?;
        }

        Ok(result as i64)
    }

    fn read_c_string(&self, addr: usize) -> Result<Vec<u8>> {
        let mut bytes = Vec::new();
        let mut offset = addr;
        // Limit to avoid runaway if the guest forgot a terminator.
        const MAX_LEN: usize = 4096;
        for _ in 0..MAX_LEN {
            let byte = self.memory.read_byte(offset)?;
            if byte == 0 {
                return Ok(bytes);
            }
            bytes.push(byte);
            offset = offset
                .checked_add(1)
                .ok_or_else(|| anyhow!("address overflow reading c-string"))?;
        }
        bail!("unterminated string starting at {addr:#x}");
    }

    fn guest_cstring(&self, addr: usize) -> Result<CString> {
        Ok(CString::new(self.read_c_string(addr)?)?)
    }

    /// Read a u64 from big-endian m68k memory
    /// m68k stores u64 as two consecutive u32 values in big-endian order
    fn read_u64_be(&self, addr: usize) -> Result<u64> {
        let hi = self.memory.read_long(addr)?;
        let lo = self.memory.read_long(addr + 4)?;
        Ok(((hi as u64) << 32) | (lo as u64))
    }

    /// Write a u64 to big-endian m68k memory
    /// m68k stores u64 as two consecutive u32 values in big-endian order
    #[allow(unused)]
    fn write_u64_be(&mut self, addr: usize, value: u64) -> Result<()> {
        let hi = (value >> 32) as u32;
        let lo = value as u32;
        self.memory.write_data(addr, &hi.to_be_bytes())?;
        self.memory.write_data(addr + 4, &lo.to_be_bytes())?;
        Ok(())
    }

    /// Read a NULL-terminated array of string pointers (e.g., argv, envp)
    /// Returns a Vec of Strings
    fn read_string_array(&self, array_addr: usize) -> Result<Vec<String>> {
        let mut strings = Vec::new();
        let mut offset = array_addr;
        const MAX_PTRS: usize = 1024; // Limit to avoid runaway

        for _i in 0..MAX_PTRS {
            // Read pointer (32-bit on m68k)
            let ptr = self
                .memory
                .read_long(offset)
                .map_err(|e| anyhow!("failed to read ptr at offset {:#x}: {}", offset, e))?
                as usize;
            if ptr == 0 {
                // NULL terminator
                break;
            }

            // Read the string at this pointer
            let c_str = self
                .guest_cstring(ptr)
                .map_err(|e| anyhow!("failed to read string at {:#x}: {}", ptr, e))?;
            let string = c_str
                .to_str()
                .map_err(|e| anyhow!("invalid UTF-8 in string array: {}", e))?
                .to_string();
            strings.push(string);

            offset = offset
                .checked_add(4)
                .ok_or_else(|| anyhow!("address overflow reading string array"))?;
        }

        if strings.len() == MAX_PTRS {
            bail!("string array exceeds maximum length at {:#x}", array_addr);
        }

        Ok(strings)
    }

    fn guest_const_ptr(&self, addr: usize, len: usize) -> Result<*const libc::c_void> {
        self.memory
            .guest_to_host(addr, len)
            .map(|p| p as *const libc::c_void)
            .ok_or_else(|| anyhow!("invalid guest buffer {addr:#x} (len {len})"))
    }

    fn guest_mut_ptr(&mut self, addr: usize, len: usize) -> Result<*mut libc::c_void> {
        self.memory
            .guest_to_host_mut(addr, len)
            .map(|p| p as *mut libc::c_void)
            .ok_or_else(|| anyhow!("invalid guest buffer {addr:#x} (len {len})"))
    }
}

/// Convert a single register value into a syscall argument type.
pub(super) trait FromReg: Sized {
    fn from_reg(v: u32) -> Self;
}

impl FromReg for u32 {
    fn from_reg(v: u32) -> Self {
        v
    }
}

impl FromReg for i32 {
    fn from_reg(v: u32) -> Self {
        v as i32
    }
}

impl FromReg for usize {
    fn from_reg(v: u32) -> Self {
        v as usize
    }
}

/// Convert slices of register values (starting at D1) into tuples of arguments.
pub(super) trait FromRegs: Sized {
    fn from_regs(regs: &[u32]) -> Self;
}

macro_rules! impl_from_regs {
    ($( $($ty:ident),+ );+ $(;)?) => {
        $(
            impl<$($ty: FromReg),+> FromRegs for ($($ty,)+) {
                fn from_regs(regs: &[u32]) -> Self {
                    let mut iter = regs.iter().copied();
                    ( $({ let v = iter.next().unwrap_or(0); $ty::from_reg(v) },)+ )
                }
            }
        )+
    };
}

impl_from_regs! {
    A;
    A, B;
    A, B, C;
    A, B, C, D;
    A, B, C, D, E;
    A, B, C, D, E, F;
}
