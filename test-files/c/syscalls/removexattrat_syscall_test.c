#include <fcntl.h>
#include <sys/syscall.h>
#include <unistd.h>

int main() {
  syscall(SYS_removexattrat, AT_FDCWD, "attr_path", "user.test");
  return 0;
}
