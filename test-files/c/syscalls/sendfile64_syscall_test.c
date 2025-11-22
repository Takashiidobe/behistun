#include <sys/sendfile.h>
#include <sys/syscall.h>
#include <unistd.h>

#ifndef SYS_sendfile64
#define SYS_sendfile64 SYS_sendfile
#endif

int main() {
  int in = syscall(SYS_memfd_create, "sendfile64_in", 0);
  int out = syscall(SYS_memfd_create, "sendfile64_out", 0);
  if (in < 0 || out < 0) {
    if (in >= 0)
      syscall(SYS_close, in);
    if (out >= 0)
      syscall(SYS_close, out);
    return 1;
  }
  syscall(SYS_write, in, "abc", 3);
  syscall(SYS_lseek, in, 0, SEEK_SET);
  long res = syscall(SYS_sendfile64, out, in, 0, 3);
  syscall(SYS_close, in);
  syscall(SYS_close, out);
  return res == 3 || res < 0 ? 0 : 1;
}
