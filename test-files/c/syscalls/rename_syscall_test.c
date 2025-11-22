#include <sys/syscall.h>
#include <unistd.h>

int main() {
  const char *oldp = "syscall_rename_old.txt";
  const char *newp = "syscall_rename_new.txt";

  int fd = syscall(SYS_creat, oldp, 0644);
  if (fd < 0) {
    return 1;
  }
  syscall(SYS_close, fd);

  if (syscall(SYS_rename, oldp, newp) < 0) {
    syscall(SYS_unlink, oldp);
    return 1;
  }

  syscall(SYS_unlink, newp);
  return 0;
}
