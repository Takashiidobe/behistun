use super::Cpu;

impl Cpu {
    pub(super) fn sys_exit(&mut self) -> ! {
        let (exit_code,): (i32,) = self.get_args();

        std::process::exit(exit_code);
    }
}
