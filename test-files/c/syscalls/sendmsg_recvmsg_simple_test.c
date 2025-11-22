#include <string.h>
#include <sys/socket.h>
#include <sys/types.h>
#include <sys/un.h>
#include <unistd.h>

// Simple test for sendmsg/recvmsg
int main() {
  int sv[2];

  // Create a socketpair for communication
  if (socketpair(AF_UNIX, SOCK_DGRAM, 0, sv) < 0) {
    return 1;
  }

  // Test: Basic sendmsg/recvmsg with iovec
  const char *msg1 = "Hello, ";
  const char *msg2 = "world!";

  // Build iovec for sending
  struct iovec send_iov[2];
  send_iov[0].iov_base = (void *)msg1;
  send_iov[0].iov_len = strlen(msg1);
  send_iov[1].iov_base = (void *)msg2;
  send_iov[1].iov_len = strlen(msg2);

  // Build msghdr for sending
  struct msghdr send_msg;
  memset(&send_msg, 0, sizeof(send_msg));
  send_msg.msg_iov = send_iov;
  send_msg.msg_iovlen = 2;

  // Send message
  ssize_t sent = sendmsg(sv[0], &send_msg, 0);
  if (sent != (ssize_t)(strlen(msg1) + strlen(msg2))) {
    close(sv[0]);
    close(sv[1]);
    return 2;
  }

  // Build iovec for receiving
  char recv_buf[128];
  struct iovec recv_iov[1];
  recv_iov[0].iov_base = recv_buf;
  recv_iov[0].iov_len = sizeof(recv_buf);

  // Build msghdr for receiving
  struct msghdr recv_msg;
  memset(&recv_msg, 0, sizeof(recv_msg));
  recv_msg.msg_iov = recv_iov;
  recv_msg.msg_iovlen = 1;

  // Receive message
  ssize_t recvd = recvmsg(sv[1], &recv_msg, 0);
  if (recvd != sent) {
    close(sv[0]);
    close(sv[1]);
    return 3;
  }

  // Verify data
  if (memcmp(recv_buf, "Hello, world!", 13) != 0) {
    close(sv[0]);
    close(sv[1]);
    return 4;
  }

  close(sv[0]);
  close(sv[1]);

  return 0;
}
