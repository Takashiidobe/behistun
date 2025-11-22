#include <stdio.h>
#include <unistd.h>

int main(void) {
  pid_t pid = getpid();
  if (pid > 0) {
    printf("getpid works\n");
  }

  pid_t ppid = getppid();
  if (ppid > 0) {
    printf("getppid works\n");
  }

  // PID should be different from PPID
  if (pid != ppid) {
    printf("pid != ppid\n");
  }

  return 0;
}
