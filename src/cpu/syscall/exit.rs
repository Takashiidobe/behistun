use super::Cpu;

impl Cpu {
    pub(super) fn sys_exit(&mut self) -> ! {
        std::process::exit(self.data_regs[1] as i32);
    }
}
