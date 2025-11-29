mod m68020;
mod syscall;

pub use m68020::{Cpu, ElfInfo};

// m68k uses a fixed TLS layout where the thread pointer lives 0x7000 bytes
// past the start of the TLS block. TLS offsets (tpoff) are negative relative
// to the thread pointer, so make sure we always leave space for this gap.
pub(super) const M68K_TLS_TCB_SIZE: usize = 0x7000;
// Give the TLS block a small pad after the thread pointer for any per-thread
// metadata the runtime might place there.
pub(super) const TLS_DATA_PAD: usize = 0x1000;

pub(super) fn align_up(value: usize, align: usize) -> usize {
    (value + align - 1) & !(align - 1)
}
