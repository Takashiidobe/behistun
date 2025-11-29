use crate::Cpu;

impl Cpu {
    pub(crate) fn sys_exit(&mut self) -> ! {
        let (exit_code,): (i32,) = self.get_args();

        std::process::exit(exit_code);
    }
}
