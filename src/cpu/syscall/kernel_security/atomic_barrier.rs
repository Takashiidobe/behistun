use anyhow::Result;

use crate::Cpu;

impl Cpu {
    /// atomic_barrier() - act as a full memory barrier on host
    pub(crate) fn sys_atomic_barrier(&self) -> Result<i64> {
        use std::sync::atomic::{Ordering, fence};
        fence(Ordering::SeqCst);
        Ok(0)
    }
}
