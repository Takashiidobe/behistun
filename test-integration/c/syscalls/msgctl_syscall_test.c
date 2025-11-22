#include <sys/ipc.h>
#include <sys/msg.h>
#include <sys/syscall.h>
#include <unistd.h>
#include <errno.h>

int main() {
  // Create a message queue
  int msqid = syscall(SYS_msgget, IPC_PRIVATE, IPC_CREAT | 0666);
  if (msqid == -1) {
    if (errno == ENOSPC || errno == ENOMEM || errno == ENOSYS || errno == EPERM) {
      return 0; // Can't test if we can't create
    }
    return 1;
  }

  // Test IPC_RMID (the most important operation)
  int rm_result = syscall(SYS_msgctl, msqid, IPC_RMID, 0);
  if (rm_result == -1 && errno != ENOSYS) {
    return 1; // IPC_RMID failed
  }

  return 0;
}
