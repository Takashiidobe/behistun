pub mod atomic_barrier;
pub mod atomic_cmpxchg_32;
pub mod capget;
pub mod capset;
pub mod init_module;
pub mod landlock_add_rule;
pub mod landlock_create_ruleset;
pub mod landlock_restrict_self;
pub mod mseal;
pub mod prctl;
pub mod syslog;

// Build host header
#[repr(C)]
struct CapUserHeader {
    version: u32,
    pid: i32,
}

// Read data from guest memory
#[repr(C)]
#[derive(Copy, Clone)]
struct CapUserData {
    effective: u32,
    permitted: u32,
    inheritable: u32,
}
