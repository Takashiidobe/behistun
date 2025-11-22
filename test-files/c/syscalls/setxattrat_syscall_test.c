#include <fcntl.h>
#include <sys/syscall.h>
#include <unistd.h>

int main() {
  syscall(SYS_setxattrat, AT_FDCWD, "attr_path", "user.test", "v", 1, 0);
  return 0;
}
