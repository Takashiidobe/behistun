#include <errno.h>
#include <fcntl.h>
#include <mqueue.h>
#include <string.h>
#include <sys/syscall.h>
#include <time.h>
#include <unistd.h>

#define TEST_QUEUE_NAME "/test_mq_queue"
#define TEST_MSG "Hello, message queue!"
#define TEST_MSG_LEN (sizeof(TEST_MSG))

int main() {
  int result;
  mqd_t mqd;
  struct mq_attr attr;
  char recv_buf[128];
  unsigned int prio;

  // Clean up any leftover queue
  syscall(SYS_mq_unlink, TEST_QUEUE_NAME);

  // Test 1: mq_open with O_CREAT
  memset(&attr, 0, sizeof(attr));
  attr.mq_flags = 0;
  attr.mq_maxmsg = 10;
  attr.mq_msgsize = 128;
  attr.mq_curmsgs = 0;

  mqd = syscall(SYS_mq_open, TEST_QUEUE_NAME, O_CREAT | O_RDWR, 0644, &attr);
  if (mqd < 0) {
    // May not be supported
    if (errno == ENOSYS || errno == ENOENT) {
      return 0;
    }
    return 1;
  }

  // Test 2: mq_getsetattr - get current attributes
  struct mq_attr current_attr;
  result = syscall(SYS_mq_getsetattr, mqd, NULL, &current_attr);
  if (result < 0) {
    syscall(SYS_mq_unlink, TEST_QUEUE_NAME);
    return 2;
  }

  // Verify attributes
  if (current_attr.mq_maxmsg != 10 || current_attr.mq_msgsize != 128) {
    syscall(SYS_mq_unlink, TEST_QUEUE_NAME);
    return 3;
  }

  // Test 3: mq_timedsend - send a message
  struct timespec timeout;
  timeout.tv_sec = time(NULL) + 5; // 5 seconds from now
  timeout.tv_nsec = 0;

  result = syscall(SYS_mq_timedsend, mqd, TEST_MSG, TEST_MSG_LEN, 0, &timeout);
  if (result < 0) {
    syscall(SYS_mq_unlink, TEST_QUEUE_NAME);
    return 4;
  }

  // Test 4: mq_timedreceive - receive the message
  memset(recv_buf, 0, sizeof(recv_buf));
  prio = 999; // Will be overwritten
  timeout.tv_sec = time(NULL) + 5;
  timeout.tv_nsec = 0;

  result = syscall(SYS_mq_timedreceive, mqd, recv_buf, sizeof(recv_buf), &prio,
                   &timeout);
  if (result < 0) {
    syscall(SYS_mq_unlink, TEST_QUEUE_NAME);
    return 5;
  }

  // Verify received message
  if (strcmp(recv_buf, TEST_MSG) != 0) {
    syscall(SYS_mq_unlink, TEST_QUEUE_NAME);
    return 6;
  }

  // Verify priority
  if (prio != 0) {
    syscall(SYS_mq_unlink, TEST_QUEUE_NAME);
    return 7;
  }

  // Test 5: Send with priority
  result = syscall(SYS_mq_timedsend, mqd, "Low priority", 13, 1, NULL);
  if (result < 0) {
    syscall(SYS_mq_unlink, TEST_QUEUE_NAME);
    return 8;
  }

  result = syscall(SYS_mq_timedsend, mqd, "High priority", 14, 10, NULL);
  if (result < 0) {
    syscall(SYS_mq_unlink, TEST_QUEUE_NAME);
    return 9;
  }

  // Test 6: Receive should get high priority first
  memset(recv_buf, 0, sizeof(recv_buf));
  prio = 0;
  result = syscall(SYS_mq_timedreceive, mqd, recv_buf, sizeof(recv_buf), &prio,
                   NULL);
  if (result < 0) {
    syscall(SYS_mq_unlink, TEST_QUEUE_NAME);
    return 10;
  }

  if (strcmp(recv_buf, "High priority") != 0 || prio != 10) {
    syscall(SYS_mq_unlink, TEST_QUEUE_NAME);
    return 11;
  }

  // Test 7: Receive low priority message
  memset(recv_buf, 0, sizeof(recv_buf));
  prio = 0;
  result = syscall(SYS_mq_timedreceive, mqd, recv_buf, sizeof(recv_buf), &prio,
                   NULL);
  if (result < 0) {
    syscall(SYS_mq_unlink, TEST_QUEUE_NAME);
    return 12;
  }

  if (strcmp(recv_buf, "Low priority") != 0 || prio != 1) {
    syscall(SYS_mq_unlink, TEST_QUEUE_NAME);
    return 13;
  }

  // Test 8: mq_getsetattr - set O_NONBLOCK
  struct mq_attr new_attr, old_attr;
  memset(&new_attr, 0, sizeof(new_attr));
  new_attr.mq_flags = O_NONBLOCK;

  result = syscall(SYS_mq_getsetattr, mqd, &new_attr, &old_attr);
  if (result < 0) {
    syscall(SYS_mq_unlink, TEST_QUEUE_NAME);
    return 14;
  }

  // Test 9: Try to receive from empty queue (should fail with EAGAIN)
  memset(recv_buf, 0, sizeof(recv_buf));
  result =
      syscall(SYS_mq_timedreceive, mqd, recv_buf, sizeof(recv_buf), NULL, NULL);
  if (result >= 0 || errno != EAGAIN) {
    syscall(SYS_mq_unlink, TEST_QUEUE_NAME);
    return 15;
  }

  // Test 10: Close and re-open existing queue
  close(mqd);

  mqd = syscall(SYS_mq_open, TEST_QUEUE_NAME, O_RDWR);
  if (mqd < 0) {
    syscall(SYS_mq_unlink, TEST_QUEUE_NAME);
    return 16;
  }

  // Test 11: mq_unlink
  close(mqd);
  result = syscall(SYS_mq_unlink, TEST_QUEUE_NAME);
  if (result < 0) {
    return 17;
  }

  // Verify queue is gone
  mqd = syscall(SYS_mq_open, TEST_QUEUE_NAME, O_RDONLY);
  if (mqd >= 0) {
    close(mqd);
    syscall(SYS_mq_unlink, TEST_QUEUE_NAME);
    return 18;
  }

  return 0; // Success!
}
