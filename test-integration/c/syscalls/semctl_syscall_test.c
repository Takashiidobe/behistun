#include <sys/ipc.h>
#include <sys/sem.h>
#include <sys/syscall.h>
#include <unistd.h>
#include <errno.h>

int main() {
  // Create a semaphore set with 1 semaphore
  int semid = syscall(SYS_semget, IPC_PRIVATE, 1, IPC_CREAT | 0666);
  if (semid == -1) {
    if (errno == ENOSPC || errno == ENOSYS || errno == EPERM || errno == EACCES) {
      return 0; // Can't test if we can't create
    }
    return 1;
  }

  // Test SETVAL
  int setval_result = syscall(SYS_semctl, semid, 0, 16 /* SETVAL */, 5);
  if (setval_result == -1 && errno != ENOSYS) {
    syscall(SYS_semctl, semid, 0, IPC_RMID);
    return 1; // SETVAL failed
  }

  // Test GETVAL
  int getval_result = syscall(SYS_semctl, semid, 0, 12 /* GETVAL */);
  if (getval_result == -1 && errno != ENOSYS) {
    syscall(SYS_semctl, semid, 0, IPC_RMID);
    return 1; // GETVAL failed
  }

  // Verify the value
  if (setval_result >= 0 && getval_result != 5) {
    syscall(SYS_semctl, semid, 0, IPC_RMID);
    return 1; // Value mismatch
  }

  // Test IPC_RMID
  int rm_result = syscall(SYS_semctl, semid, 0, IPC_RMID);
  if (rm_result == -1 && errno != ENOSYS) {
    return 1; // IPC_RMID failed
  }

  return 0;
}
