#include <sys/inotify.h>
#include <sys/syscall.h>
#include <unistd.h>

int main() {
  int fd = syscall(SYS_inotify_init);
  if (fd >= 0) {
    int wd = syscall(SYS_inotify_add_watch, fd, ".", IN_MODIFY);
    syscall(SYS_inotify_rm_watch, fd, wd);
    syscall(SYS_close, fd);
  }
  return 0;
}
