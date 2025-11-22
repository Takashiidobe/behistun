#include <fcntl.h>
#include <sys/syscall.h>
#include <unistd.h>

int main() {
  char buf[4];
  syscall(SYS_getxattrat, AT_FDCWD, "attr_path", "user.test", buf, sizeof(buf));
  return 0;
}
