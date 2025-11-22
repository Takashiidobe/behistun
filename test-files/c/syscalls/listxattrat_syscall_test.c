#include <fcntl.h>
#include <sys/syscall.h>
#include <unistd.h>

int main() {
  char buf[8];
  syscall(SYS_listxattrat, AT_FDCWD, "attr_path", buf, sizeof(buf));
  return 0;
}
