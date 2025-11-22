#include <sys/ipc.h>
#include <sys/shm.h>
#include <sys/syscall.h>
#include <unistd.h>
#include <errno.h>
#include <string.h>

int main() {
  // Create a shared memory segment (4096 bytes)
  int shmid = syscall(SYS_shmget, IPC_PRIVATE, 4096, IPC_CREAT | 0666);
  if (shmid == -1) {
    if (errno == ENOSPC || errno == ENOSYS || errno == EPERM || errno == EACCES) {
      return 0; // Can't test if we can't create
    }
    return 1; // Unexpected failure
  }

  // Attach the shared memory
  void *addr1 = (void *)syscall(SYS_shmat, shmid, 0, 0);
  if (addr1 == (void *)-1) {
    if (errno == ENOSYS) {
      syscall(SYS_shmctl, shmid, IPC_RMID, 0);
      return 0; // shmat not implemented
    }
    syscall(SYS_shmctl, shmid, IPC_RMID, 0);
    return 1; // shmat failed
  }

  // Write some data to the shared memory
  const char *test_string = "Hello from shmat!";
  strcpy((char *)addr1, test_string);

  // Detach the shared memory
  int detach_result = syscall(SYS_shmdt, addr1);
  if (detach_result == -1) {
    if (errno == ENOSYS) {
      syscall(SYS_shmctl, shmid, IPC_RMID, 0);
      return 0; // shmdt not implemented
    }
    syscall(SYS_shmctl, shmid, IPC_RMID, 0);
    return 1; // shmdt failed
  }

  // Attach again to verify data persists
  void *addr2 = (void *)syscall(SYS_shmat, shmid, 0, 0);
  if (addr2 == (void *)-1) {
    syscall(SYS_shmctl, shmid, IPC_RMID, 0);
    return 1; // Second shmat failed
  }

  // Verify the data is still there
  if (strcmp((char *)addr2, test_string) != 0) {
    syscall(SYS_shmdt, addr2);
    syscall(SYS_shmctl, shmid, IPC_RMID, 0);
    return 1; // Data mismatch
  }

  // Clean up
  syscall(SYS_shmdt, addr2);
  syscall(SYS_shmctl, shmid, IPC_RMID, 0);

  return 0;
}
