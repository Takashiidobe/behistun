use std::{collections::HashMap, sync::LazyLock};

/// Mapping from m68k Linux syscall numbers to x86_64 Linux syscall numbers.
///
/// Built by pairing syscalls with the same name across the two architectures.
/// Any m68k syscall missing on x86_64 is omitted; callers should treat an
/// absent entry as unsupported.
pub fn m68k_to_x86_64_syscall(number: u32) -> Option<u32> {
    M68K_TO_X86_64.get(&number).copied()
}

fn table() -> HashMap<u32, u32> {
    // Generated from m68k-syscalls.txt and x86-64-syscalls.txt
    // Format: (m68k_syscall_num, x86_64_syscall_num)
    [
        (0, 219),  // restart_syscall
        (1, 60),   // exit
        (2, 57),   // fork
        (3, 0),    // read
        (4, 1),    // write
        (5, 2),    // open
        (6, 3),    // close
        (7, 61),   // waitpid is translated to wait4
        (8, 85),   // creat
        (9, 86),   // link
        (10, 87),  // unlink
        (11, 59),  // execve
        (12, 80),  // chdir
        (13, 201), // time
        (14, 133), // mknod
        (15, 90),  // chmod
        (16, 92),  // chown
        // 18: oldstat - m68k only
        (19, 8),   // lseek
        (20, 39),  // getpid
        (21, 165), // mount
        // 22: umount - m68k only
        (23, 105), // setuid
        (24, 102), // getuid
        // 25: stime - m68k only
        (26, 101), // ptrace
        (27, 37),  // alarm
        // 28: oldfstat - m68k only
        (29, 34),  // pause
        (30, 132), // utime
        (33, 21),  // access
        // 34: nice - m68k only
        (36, 162), // sync
        (37, 62),  // kill
        (38, 82),  // rename
        (39, 83),  // mkdir
        (40, 84),  // rmdir
        (41, 32),  // dup
        (42, 22),  // pipe
        (43, 100), // times
        (45, 12),  // brk
        (46, 106), // setgid
        (47, 104), // getgid
        // 48: signal - m68k only
        (49, 107), // geteuid
        (50, 108), // getegid
        (51, 163), // acct
        (52, 166), // umount2
        (54, 16),  // ioctl
        (55, 72),  // fcntl
        (57, 109), // setpgid
        (60, 95),  // umask
        (61, 161), // chroot
        (62, 136), // ustat
        (63, 33),  // dup2
        (64, 110), // getppid
        (65, 111), // getpgrp
        (66, 112), // setsid
        // 67: sigaction - m68k only
        // 68: sgetmask - m68k only
        // 69: ssetmask - m68k only
        (70, 113), // setreuid
        (71, 114), // setregid
        // 72: sigsuspend - m68k only
        // 73: sigpending - m68k only
        (74, 170), // sethostname
        (75, 160), // setrlimit
        (76, 97),  // getrlimit
        (77, 98),  // getrusage
        (78, 96),  // gettimeofday
        (79, 164), // settimeofday
        (80, 115), // getgroups
        (81, 116), // setgroups
        (82, 23),  // select
        (83, 88),  // symlink
        // 84: oldlstat - m68k only
        (85, 89),  // readlink
        (86, 134), // uselib
        (87, 167), // swapon
        (88, 169), // reboot
        // 89: readdir - m68k only
        (90, 9),    // mmap
        (91, 11),   // munmap
        (92, 76),   // truncate
        (93, 77),   // ftruncate
        (94, 91),   // fchmod
        (95, 93),   // fchown
        (96, 140),  // getpriority
        (97, 141),  // setpriority
        (99, 137),  // statfs
        (100, 138), // fstatfs
        // 102: socketcall - m68k only
        (103, 103), // syslog
        (104, 38),  // setitimer
        (105, 36),  // getitimer
        (106, 4),   // stat
        (107, 6),   // lstat
        (108, 5),   // fstat
        (111, 153), // vhangup
        (114, 61),  // wait4
        (115, 168), // swapoff
        (116, 99),  // sysinfo
        // 117: ipc - m68k only
        (118, 74), // fsync
        // 119: sigreturn - m68k only
        (120, 56),  // clone
        (121, 171), // setdomainname
        (122, 63),  // uname
        // 123: cacheflush - m68k only
        (124, 159), // adjtimex
        (125, 10),  // mprotect
        // 126: sigprocmask - m68k only
        (127, 174), // create_module
        (128, 175), // init_module
        (129, 176), // delete_module
        (130, 177), // get_kernel_syms
        (131, 179), // quotactl
        (132, 121), // getpgid
        (133, 81),  // fchdir
        // 134: bdflush - m68k only
        (135, 139), // sysfs
        (136, 135), // personality
        (138, 122), // setfsuid
        (139, 123), // setfsgid
        (140, 8),   // _llseek -> lseek on x86_64 (handled in dispatcher)
        (141, 78),  // getdents
        // 142: _newselect - m68k only
        (143, 73),  // flock
        (144, 26),  // msync
        (145, 19),  // readv
        (146, 20),  // writev
        (147, 124), // getsid
        (148, 75),  // fdatasync
        (149, 156), // _sysctl
        (150, 149), // mlock
        (151, 150), // munlock
        (152, 151), // mlockall
        (153, 152), // munlockall
        (154, 142), // sched_setparam
        (155, 143), // sched_getparam
        (156, 144), // sched_setscheduler
        (157, 145), // sched_getscheduler
        (158, 24),  // sched_yield
        (159, 146), // sched_get_priority_max
        (160, 147), // sched_get_priority_min
        (161, 148), // sched_rr_get_interval
        (162, 35),  // nanosleep
        (163, 25),  // mremap
        (164, 117), // setresuid
        (165, 118), // getresuid
        // 166: getpagesize - m68k only, forwarded to sysconf(_SC_PAGESIZE);
        (167, 178), // query_module
        (168, 7),   // poll
        (169, 180), // nfsservctl
        (170, 119), // setresgid
        (171, 120), // getresgid
        (172, 157), // prctl
        (173, 15),  // rt_sigreturn
        (174, 13),  // rt_sigaction
        (175, 14),  // rt_sigprocmask
        (176, 127), // rt_sigpending
        (177, 128), // rt_sigtimedwait
        (178, 129), // rt_sigqueueinfo
        (179, 130), // rt_sigsuspend
        (180, 17),  // pread64
        (181, 18),  // pwrite64
        (182, 94),  // lchown
        (183, 79),  // getcwd
        (184, 125), // capget
        (185, 126), // capset
        (186, 131), // sigaltstack
        (187, 40),  // sendfile
        (188, 181), // getpmsg
        (189, 182), // putpmsg
        (190, 58),  // vfork
        (191, 97),  // ugetrlimit - m68k only, maps to getrlimit on x86_64
        (192, 9),   // mmap2 - maps to mmap on x86_64 (special handling in code)
        (193, 76),  // truncate64 - maps to truncate on x86_64
        (194, 77),  // ftruncate64 - maps to ftruncate on x86_64
        (195, 4),   // stat64 - maps to stat on x86_64
        (196, 6),   // lstat64 - maps to lstat on x86_64
        (197, 5),   // fstat64 - maps to fstat on x86_64
        (198, 92),  // chown32 - m68k only
        (199, 102), // getuid32 - m68k only
        (200, 104), // getgid32 - m68k only
        (201, 107), // geteuid32 - m68k only
        (202, 108), // getegid32 - m68k only
        (203, 113), // setreuid32 - m68k only
        (204, 114), // setregid32 - m68k only
        (205, 115), // getgroups32 - m68k only
        (206, 116), // setgroups32 - m68k only
        (207, 93),  // fchown32 - m68k only
        (208, 117), // setresuid32 - m68k only
        (209, 118), // getresuid32 - m68k only
        (210, 119), // setresgid32 - m68k only
        (211, 120), // getresgid32 - m68k only
        (212, 94),  // lchown32 - m68k only
        (213, 105), // setuid32 - m68k only
        (214, 106), // setgid32 - m68k only
        (215, 122), // setfsuid32 - m68k only
        (216, 123), // setfsgid32 - m68k only
        (217, 155), // pivot_root
        (220, 217), // getdents64
        (221, 186), // gettid
        (222, 200), // tkill
        (223, 188), // setxattr
        (224, 189), // lsetxattr
        (225, 190), // fsetxattr
        (226, 191), // getxattr
        (227, 192), // lgetxattr
        (228, 193), // fgetxattr
        (229, 194), // listxattr
        (230, 195), // llistxattr
        (231, 196), // flistxattr
        (232, 197), // removexattr
        (233, 198), // lremovexattr
        (234, 199), // fremovexattr
        (235, 202), // futex
        (236, 40),  // sendfile64 - maps to sendfile on x86_64
        (237, 27),  // mincore
        (238, 28),  // madvise
        (239, 72),  // fcntl64 - maps to fcntl on x86_64
        (240, 187), // readahead
        (241, 206), // io_setup
        (242, 207), // io_destroy
        (243, 208), // io_getevents
        (244, 209), // io_submit
        (245, 210), // io_cancel
        (246, 221), // fadvise64
        (247, 231), // exit_group
        (248, 212), // lookup_dcookie
        (249, 213), // epoll_create
        (250, 233), // epoll_ctl
        (251, 232), // epoll_wait
        (252, 216), // remap_file_pages
        (253, 218), // set_tid_address
        (254, 222), // timer_create
        (255, 223), // timer_settime
        (256, 224), // timer_gettime
        (257, 225), // timer_getoverrun
        (258, 226), // timer_delete
        (259, 227), // clock_settime
        (260, 228), // clock_gettime
        (261, 229), // clock_getres
        (262, 230), // clock_nanosleep
        (263, 137), // statfs64 - maps to statfs on x86_64
        (264, 138), // fstatfs64 - maps to fstatfs on x86_64
        (265, 234), // tgkill
        (266, 235), // utimes
        (267, 221), // fadvise64_64 - m68k only, maps to fadvise on x86_64
        (268, 237), // mbind
        (269, 239), // get_mempolicy
        (270, 238), // set_mempolicy
        (271, 240), // mq_open
        (272, 241), // mq_unlink
        (273, 242), // mq_timedsend
        (274, 243), // mq_timedreceive
        (275, 244), // mq_notify
        (276, 245), // mq_getsetattr
        (277, 247), // waitid
        (279, 248), // add_key
        (280, 249), // request_key
        (281, 250), // keyctl
        (282, 251), // ioprio_set
        (283, 252), // ioprio_get
        (284, 253), // inotify_init
        (285, 254), // inotify_add_watch
        (286, 255), // inotify_rm_watch
        (287, 256), // migrate_pages
        (288, 257), // openat
        (289, 258), // mkdirat
        (290, 259), // mknodat
        (291, 260), // fchownat
        (292, 261), // futimesat
        (293, 262), // fstatat64 - maps to newfstatat on x86_64
        (294, 263), // unlinkat
        (295, 264), // renameat
        (296, 265), // linkat
        (297, 266), // symlinkat
        (298, 267), // readlinkat
        (299, 268), // fchmodat
        (300, 269), // faccessat
        (301, 270), // pselect6
        (302, 271), // ppoll
        (303, 272), // unshare
        (304, 273), // set_robust_list
        (305, 274), // get_robust_list
        (306, 275), // splice
        (307, 277), // sync_file_range
        (308, 276), // tee
        (309, 278), // vmsplice
        (310, 279), // move_pages
        (311, 203), // sched_setaffinity
        (312, 204), // sched_getaffinity
        (313, 246), // kexec_load
        (314, 309), // getcpu
        (315, 281), // epoll_pwait
        (316, 280), // utimensat
        (317, 282), // signalfd
        (318, 283), // timerfd_create
        (319, 284), // eventfd
        (320, 285), // fallocate
        (321, 286), // timerfd_settime
        (322, 287), // timerfd_gettime
        (323, 289), // signalfd4
        (324, 290), // eventfd2
        (325, 291), // epoll_create1
        (326, 292), // dup3
        (327, 293), // pipe2
        (328, 294), // inotify_init1
        (329, 295), // preadv
        (330, 296), // pwritev
        (331, 297), // rt_tgsigqueueinfo
        (332, 298), // perf_event_open
        (333, 211), // get_thread_area
        (334, 205), // set_thread_area
        // 335: atomic_cmpxchg_32 - m68k only implemented by emulator
        // 336: atomic_barrier - m68k only implemented by emulator
        (337, 300), // fanotify_init
        (338, 301), // fanotify_mark
        (339, 302), // prlimit64
        (340, 303), // name_to_handle_at
        (341, 304), // open_by_handle_at
        (342, 305), // clock_adjtime
        (343, 306), // syncfs
        (344, 308), // setns
        (345, 310), // process_vm_readv
        (346, 311), // process_vm_writev
        (347, 312), // kcmp
        (348, 313), // finit_module
        (349, 314), // sched_setattr
        (350, 315), // sched_getattr
        (351, 316), // renameat2
        (352, 318), // getrandom
        (353, 319), // memfd_create
        (354, 321), // bpf
        (355, 322), // execveat
        (356, 41),  // socket
        (357, 53),  // socketpair
        (358, 49),  // bind
        (359, 42),  // connect
        (360, 50),  // listen
        (361, 288), // accept4
        (362, 55),  // getsockopt
        (363, 54),  // setsockopt
        (364, 51),  // getsockname
        (365, 52),  // getpeername
        (366, 44),  // sendto
        (367, 46),  // sendmsg
        (368, 45),  // recvfrom
        (369, 47),  // recvmsg
        (370, 48),  // shutdown
        (371, 299), // recvmmsg
        (372, 307), // sendmmsg
        (373, 323), // userfaultfd
        (374, 324), // membarrier
        (375, 325), // mlock2
        (376, 326), // copy_file_range
        (377, 327), // preadv2
        (378, 328), // pwritev2
        (379, 332), // statx
        (380, 317), // seccomp
        (381, 329), // pkey_mprotect
        (382, 330), // pkey_alloc
        (383, 331), // pkey_free
        (384, 334), // rseq
        (393, 64),  // semget
        (394, 66),  // semctl
        (395, 29),  // shmget
        (396, 31),  // shmctl
        (397, 30),  // shmat
        (398, 67),  // shmdt
        (399, 68),  // msgget
        (400, 69),  // msgsnd
        (401, 70),  // msgrcv
        (402, 71),  // msgctl
        // m68k uclibc uses 64-bit time_t, so *_time64 syscalls map to regular x86-64 versions
        (403, 228), // clock_gettime64 -> clock_gettime (x86-64 uses 64-bit time_t)
        (404, 227), // clock_settime64 -> clock_settime
        (405, 305), // clock_adjtime64 -> clock_adjtime
        (406, 229), // clock_getres_time64 -> clock_getres
        (407, 230), // clock_nanosleep_time64 -> clock_nanosleep
        (408, 224), // timer_gettime64 -> timer_gettime
        (409, 223), // timer_settime64 -> timer_settime
        (410, 287), // timerfd_gettime64 -> timerfd_gettime
        (411, 286), // timerfd_settime64 -> timerfd_settime
        (412, 280), // utimensat_time64 -> utimensat
        (413, 270), // pselect6_time64 -> pselect6
        (414, 271), // ppoll_time64 -> ppoll
        // 416: io_pgetevents_time64 - no x86-64 equivalent
        (417, 299), // recvmmsg_time64 -> recvmmsg
        (418, 242), // mq_timedsend_time64 -> mq_timedsend
        (419, 243), // mq_timedreceive_time64 -> mq_timedreceive
        (420, 220), // semtimedop_time64 -> semtimedop
        (421, 128), // rt_sigtimedwait_time64 -> rt_sigtimedwait
        (422, 202), // futex_time64 -> futex
        (423, 148), // sched_rr_get_interval_time64 -> sched_rr_get_interval
        (424, 424), // pidfd_send_signal
        (425, 425), // io_uring_setup
        (426, 426), // io_uring_enter
        (427, 427), // io_uring_register
        (428, 428), // open_tree
        (429, 429), // move_mount
        (430, 430), // fsopen
        (431, 431), // fsconfig
        (432, 432), // fsmount
        (433, 433), // fspick
        (434, 434), // pidfd_open
        (435, 435), // clone3
        (436, 436), // close_range
        (437, 437), // openat2
        (438, 438), // pidfd_getfd
        (439, 439), // faccessat2
        (440, 440), // process_madvise
        (441, 441), // epoll_pwait2
        (442, 442), // mount_setattr
        (443, 443), // quotactl_fd
        (444, 444), // landlock_create_ruleset
        (445, 445), // landlock_add_rule
        (446, 446), // landlock_restrict_self
        (448, 448), // process_mrelease
        (449, 449), // futex_waitv
        (450, 450), // set_mempolicy_home_node
        (451, 451), // cachestat
        (452, 452), // fchmodat2
        (453, 453), // map_shadow_stack
        (454, 454), // futex_wake
        (455, 455), // futex_wait
        (456, 456), // futex_requeue
        (457, 457), // statmount
        (458, 458), // listmount
        (459, 459), // lsm_get_self_attr
        (460, 460), // lsm_set_self_attr
        (461, 461), // lsm_list_modules
        (462, 462), // mseal
        (463, 463), // setxattrat
        (464, 464), // getxattrat
        (465, 465), // listxattrat
        (466, 466), // removexattrat
        (467, 467), // open_tree_attr
    ]
    .into_iter()
    .collect()
}

static M68K_TO_X86_64: LazyLock<HashMap<u32, u32>> = LazyLock::new(table);
