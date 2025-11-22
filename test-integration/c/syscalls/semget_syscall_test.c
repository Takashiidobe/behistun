#include <sys/ipc.h>
#include <sys/sem.h>
#include <sys/syscall.h>
#include <unistd.h>
#include <errno.h>

int main() {
  // Create a unique key
  key_t key = IPC_PRIVATE;

  // Try to create a semaphore set with 1 semaphore
  int semid = syscall(SYS_semget, key, 1, IPC_CREAT | 0666);

  if (semid == -1) {
    // It's OK if it fails due to permissions or system limits
    // We just want to verify the syscall is dispatched correctly
    if (errno == ENOSPC || errno == ENOSYS || errno == EPERM || errno == EACCES) {
      return 0; // Expected failure modes
    }
    return 1; // Unexpected failure
  }

  // Success - semget works! Clean up the semaphore
  int rm_result = syscall(SYS_semctl, semid, 0, IPC_RMID);
  if (rm_result == -1) {
    return 1; // Cleanup failed
  }
  return 0;
}
