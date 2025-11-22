#include <errno.h>
#include <stdio.h>
#include <string.h>
#include <sys/socket.h>
#include <sys/types.h>
#include <sys/un.h>
#include <unistd.h>

// Test sendmsg/recvmsg with Unix domain sockets - with debug output
int main() {
  int sv[2];

  // Create a socketpair for communication
  printf("Creating socketpair...\n");
  if (socketpair(AF_UNIX, SOCK_DGRAM, 0, sv) < 0) {
    printf("socketpair failed: errno=%d\n", errno);
    return 1;
  }
  printf("socketpair succeeded: sv[0]=%d, sv[1]=%d\n", sv[0], sv[1]);

  // Test 1: Basic sendmsg/recvmsg with iovec
  const char *msg1 = "Hello, ";
  const char *msg2 = "world!";

  // Build iovec for sending
  struct iovec send_iov[2];
  send_iov[0].iov_base = (void *)msg1;
  send_iov[0].iov_len = strlen(msg1);
  send_iov[1].iov_base = (void *)msg2;
  send_iov[1].iov_len = strlen(msg2);

  printf("Prepared iovecs: iov[0].len=%u, iov[1].len=%u\n",
         (unsigned)send_iov[0].iov_len, (unsigned)send_iov[1].iov_len);

  // Build msghdr for sending
  struct msghdr send_msg;
  memset(&send_msg, 0, sizeof(send_msg));
  send_msg.msg_iov = send_iov;
  send_msg.msg_iovlen = 2;

  printf("Calling sendmsg...\n");
  // Send message
  ssize_t sent = sendmsg(sv[0], &send_msg, 0);
  if (sent < 0) {
    printf("sendmsg failed: errno=%d, sent=%ld\n", errno, (long)sent);
    close(sv[0]);
    close(sv[1]);
    return 2;
  }
  printf("sendmsg succeeded: sent=%ld bytes\n", (long)sent);

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

  printf("Calling recvmsg...\n");
  // Receive message
  ssize_t recvd = recvmsg(sv[1], &recv_msg, 0);
  if (recvd < 0) {
    printf("recvmsg failed: errno=%d, recvd=%ld\n", errno, (long)recvd);
    close(sv[0]);
    close(sv[1]);
    return 3;
  }
  printf("recvmsg succeeded: recvd=%ld bytes\n", (long)recvd);

  // Verify data
  recv_buf[recvd] = '\0';
  printf("Received data: '%s'\n", recv_buf);

  if (recvd != sent) {
    printf("Size mismatch: sent=%ld, recvd=%ld\n", (long)sent, (long)recvd);
    close(sv[0]);
    close(sv[1]);
    return 4;
  }

  if (memcmp(recv_buf, "Hello, world!", 13) != 0) {
    printf("Data mismatch\n");
    close(sv[0]);
    close(sv[1]);
    return 5;
  }

  printf("Test passed!\n");
  close(sv[0]);
  close(sv[1]);

  return 0;
}
