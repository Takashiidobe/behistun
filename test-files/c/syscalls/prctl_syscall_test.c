#include <sys/prctl.h>
#include <sys/syscall.h>
#include <unistd.h>

int main() {
  int sig = 0;
  long res = syscall(SYS_prctl, PR_GET_PDEATHSIG, &sig, 0, 0, 0);
  return res == 0 ? 0 : 1;
}
