#include <fcntl.h>
#include <sys/syscall.h>
#include <time.h>
#include <unistd.h>

int main() {
  syscall(SYS_utimensat_time64, AT_FDCWD, ".",
          (struct timespec[]){{0, 0}, {0, 0}}, 0);
  return 0;
}
