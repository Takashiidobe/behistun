#include <sys/ipc.h>
#include <sys/msg.h>
#include <sys/syscall.h>
#include <unistd.h>
#include <errno.h>
#include <string.h>

struct msgbuf {
  long mtype;
  char mtext[64];
};

int main() {
  // Create a message queue
  int msqid = syscall(SYS_msgget, IPC_PRIVATE, IPC_CREAT | 0666);
  if (msqid == -1) {
    if (errno == ENOSPC || errno == ENOMEM || errno == ENOSYS || errno == EPERM) {
      return 0; // Can't test if we can't create
    }
    return 1;
  }

  // Send a message
  struct msgbuf send_buf;
  send_buf.mtype = 1;
  strcpy(send_buf.mtext, "Hello, IPC!");

  int send_result = syscall(SYS_msgsnd, msqid, &send_buf, strlen(send_buf.mtext) + 1, 0);
  if (send_result == -1 && errno != ENOSYS) {
    syscall(SYS_msgctl, msqid, IPC_RMID, 0);
    return 1; // msgsnd failed
  }

  // Receive the message
  struct msgbuf recv_buf;
  memset(&recv_buf, 0, sizeof(recv_buf));

  int recv_result = syscall(SYS_msgrcv, msqid, &recv_buf, sizeof(recv_buf.mtext), 1, 0);
  if (recv_result == -1 && errno != ENOSYS) {
    syscall(SYS_msgctl, msqid, IPC_RMID, 0);
    return 1; // msgrcv failed
  }

  // Verify the message
  if (recv_result >= 0) {
    if (recv_buf.mtype != 1 || strcmp(recv_buf.mtext, "Hello, IPC!") != 0) {
      syscall(SYS_msgctl, msqid, IPC_RMID, 0);
      return 1; // Message mismatch
    }
  }

  // Clean up
  syscall(SYS_msgctl, msqid, IPC_RMID, 0);
  return 0;
}
