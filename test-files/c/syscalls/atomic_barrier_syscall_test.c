#include <sys/syscall.h>
#include <unistd.h>

int main() {
  long res = syscall(__NR_atomic_barrier);
  return res;
}
