#include <stdio.h>
#include <string.h>
#include <sys/socket.h>
#include <sys/types.h>
#include <sys/un.h>
#include <unistd.h>

// Test sendmsg/recvmsg with Unix domain sockets
int main() {
  int sv[2];

  // Create a socketpair for communication
  if (socketpair(AF_UNIX, SOCK_DGRAM, 0, sv) < 0) {
    return 1;
  }

  // Test 1: Basic sendmsg/recvmsg with iovec
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

  // Test 2: sendmsg/recvmsg with NULL iovec (edge case)
  struct msghdr empty_msg;
  memset(&empty_msg, 0, sizeof(empty_msg));
  empty_msg.msg_iov = NULL;
  empty_msg.msg_iovlen = 0;

  // This should succeed with 0 bytes sent
  sent = sendmsg(sv[0], &empty_msg, 0);
  if (sent != 0) {
    close(sv[0]);
    close(sv[1]);
    return 5;
  }

  // Test 3: Single small message
  const char *test3_data = "Test message 3";
  struct iovec test3_iov;
  test3_iov.iov_base = (void *)test3_data;
  test3_iov.iov_len = strlen(test3_data);

  struct msghdr test3_msg;
  memset(&test3_msg, 0, sizeof(test3_msg));
  test3_msg.msg_iov = &test3_iov;
  test3_msg.msg_iovlen = 1;

  sent = sendmsg(sv[0], &test3_msg, 0);
  if (sent != (ssize_t)strlen(test3_data)) {
    close(sv[0]);
    close(sv[1]);
    return 6;
  }

  char verify_buf[32];
  struct iovec verify_iov;
  verify_iov.iov_base = verify_buf;
  verify_iov.iov_len = sizeof(verify_buf);

  struct msghdr verify_msg;
  memset(&verify_msg, 0, sizeof(verify_msg));
  verify_msg.msg_iov = &verify_iov;
  verify_msg.msg_iovlen = 1;

  recvd = recvmsg(sv[1], &verify_msg, 0);
  if (recvd != sent || memcmp(verify_buf, test3_data, sent) != 0) {
    close(sv[0]);
    close(sv[1]);
    return 7;
  }

  close(sv[0]);
  close(sv[1]);

  return 0;
}
