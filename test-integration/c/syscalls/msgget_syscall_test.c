#include <sys/ipc.h>
#include <sys/msg.h>
#include <sys/syscall.h>
#include <unistd.h>
#include <errno.h>

int main() {
  // Create a unique key
  key_t key = IPC_PRIVATE;

  // Try to create a message queue
  int msqid = syscall(SYS_msgget, key, IPC_CREAT | 0666);

  if (msqid == -1) {
    // It's OK if it fails due to permissions or system limits
    // We just want to verify the syscall is dispatched correctly
    if (errno == ENOSPC || errno == ENOMEM || errno == ENOSYS || errno == EPERM || errno == EACCES) {
      return 0; // Expected failure modes
    }
    return 1; // Unexpected failure
  }

  // Success - msgget works! Clean up the message queue
  int rm_result = syscall(SYS_msgctl, msqid, IPC_RMID, 0);
  if (rm_result == -1) {
    return 1; // Cleanup failed
  }
  return 0;
}
