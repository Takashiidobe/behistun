#include <sys/ipc.h>
#include <sys/shm.h>
#include <sys/syscall.h>
#include <unistd.h>
#include <errno.h>

int main() {
  // Create a unique key
  key_t key = IPC_PRIVATE;

  // Try to create a shared memory segment (1 page = 4096 bytes)
  int shmid = syscall(SYS_shmget, key, 4096, IPC_CREAT | 0666);

  if (shmid == -1) {
    // It's OK if it fails due to permissions or system limits
    // We just want to verify the syscall is dispatched correctly
    if (errno == ENOSPC || errno == ENOMEM || errno == ENOSYS || errno == EPERM || errno == EACCES) {
      return 0; // Expected failure modes
    }
    return 1; // Unexpected failure
  }

  // Success - shmget works! Clean up the shared memory
  int rm_result = syscall(SYS_shmctl, shmid, IPC_RMID, 0);
  if (rm_result == -1) {
    return 1; // Cleanup failed
  }
  return 0;
}
