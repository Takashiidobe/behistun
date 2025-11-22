#include <linux/rseq.h>
#include <sys/syscall.h>
#include <unistd.h>

int main() {
  struct rseq rs = {0};
  syscall(SYS_rseq, &rs, sizeof(rs), 0, 0);
  return 0;
}
