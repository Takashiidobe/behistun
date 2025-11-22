#include <stdint.h>
#include <sys/syscall.h>
#include <unistd.h>

int main() {
  uint32_t val = 1;
  long prev = syscall(__NR_atomic_cmpxchg_32, &val, 1, 5);
  if (prev != 1)
    return 1;
  return val == 5 ? 0 : 1;
}
