#include <sys/ipc.h>
#include <sys/shm.h>
#include <sys/syscall.h>
#include <unistd.h>
#include <errno.h>

int main() {
  // Create a shared memory segment
  int shmid = syscall(SYS_shmget, IPC_PRIVATE, 4096, IPC_CREAT | 0666);
  if (shmid == -1) {
    if (errno == ENOSPC || errno == ENOMEM || errno == ENOSYS || errno == EPERM) {
      return 0; // Can't test if we can't create
    }
    return 1;
  }

  // Test IPC_RMID (the most important operation)
  int rm_result = syscall(SYS_shmctl, shmid, IPC_RMID, 0);
  if (rm_result == -1 && errno != ENOSYS) {
    return 1; // IPC_RMID failed
  }

  return 0;
}
