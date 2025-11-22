#include <fcntl.h>
#include <sys/syscall.h>
#include <unistd.h>

int main() {
  int fd = syscall(SYS_open, "renameat2_old.txt", O_CREAT | O_RDWR, 0644);
  if (fd >= 0) {
    syscall(SYS_close, fd);
    syscall(SYS_renameat2, AT_FDCWD, "renameat2_old.txt", AT_FDCWD,
            "renameat2_new.txt", 0);
    syscall(SYS_unlink, "renameat2_new.txt");
  }
  return 0;
}
