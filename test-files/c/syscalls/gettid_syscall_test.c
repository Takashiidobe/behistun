#include <sys/syscall.h>
#include <unistd.h>

int main() {
  long tid = syscall(SYS_gettid);
  return tid > 0 ? 0 : 1;
}
