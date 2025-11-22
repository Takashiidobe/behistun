#include <fcntl.h>
#include <stdio.h>
#include <string.h>
#include <sys/uio.h>
#include <unistd.h>

int main(void) {
  int fd = open("/tmp/iovec_test.txt", O_CREAT | O_WRONLY | O_TRUNC, 0644);
  if (fd < 0) {
    perror("open");
    return 1;
  }

  // Test writev with multiple buffers
  struct iovec iov[3];
  char buf1[] = "Hello, ";
  char buf2[] = "World";
  char buf3[] = "!\n";

  iov[0].iov_base = buf1;
  iov[0].iov_len = strlen(buf1);
  iov[1].iov_base = buf2;
  iov[1].iov_len = strlen(buf2);
  iov[2].iov_base = buf3;
  iov[2].iov_len = strlen(buf3);

  ssize_t written = writev(fd, iov, 3);
  if (written < 0) {
    perror("writev");
    close(fd);
    unlink("/tmp/iovec_test.txt");
    return 1;
  }

  printf("writev works\n");
  close(fd);

  // Test readv
  fd = open("/tmp/iovec_test.txt", O_RDONLY);
  if (fd < 0) {
    perror("open for read");
    unlink("/tmp/iovec_test.txt");
    return 1;
  }

  char rbuf1[10], rbuf2[10], rbuf3[10];
  struct iovec riov[3];
  riov[0].iov_base = rbuf1;
  riov[0].iov_len = 7;
  riov[1].iov_base = rbuf2;
  riov[1].iov_len = 5;
  riov[2].iov_base = rbuf3;
  riov[2].iov_len = 2;

  ssize_t nread = readv(fd, riov, 3);
  if (nread > 0) {
    printf("readv works\n");
  }

  close(fd);
  unlink("/tmp/iovec_test.txt");

  return 0;
}
