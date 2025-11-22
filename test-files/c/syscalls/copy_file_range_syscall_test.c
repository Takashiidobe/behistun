#include <sys/syscall.h>
#include <unistd.h>

int main() {
  int in = syscall(SYS_memfd_create, "cfr_in", 0);
  int out = syscall(SYS_memfd_create, "cfr_out", 0);
  if (in >= 0 && out >= 0) {
    syscall(SYS_write, in, "abc", 3);
    syscall(SYS_lseek, in, 0, SEEK_SET);
    syscall(SYS_copy_file_range, in, 0, out, 0, 3, 0);
  }
  if (in >= 0)
    syscall(SYS_close, in);
  if (out >= 0)
    syscall(SYS_close, out);
  return 0;
}
