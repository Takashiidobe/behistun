#include <assert.h>
#include <fcntl.h>
#include <stdio.h>
#include <string.h>
#include <unistd.h>

int main(void) {
  const char *path = "/tmp/tmp_rw_append.txt";
  int fd = open(path, O_CREAT | O_TRUNC | O_RDWR, 0644);
  assert(fd >= 0);
  const char *msg1 = "hello";
  assert(write(fd, msg1, strlen(msg1)) == (ssize_t)strlen(msg1));
  off_t pos = lseek(fd, 0, SEEK_SET);
  assert(pos == 0);
  char buf[16] = {0};
  assert(read(fd, buf, sizeof(buf)) == (ssize_t)strlen(msg1));
  printf("%s\n", buf);
  const char *msg2 = " world";
  assert(lseek(fd, 0, SEEK_END) >= 0);
  assert(write(fd, msg2, strlen(msg2)) == (ssize_t)strlen(msg2));
  lseek(fd, 0, SEEK_SET);
  memset(buf, 0, sizeof(buf));
  assert(read(fd, buf, sizeof(buf)) == (ssize_t)(strlen(msg1) + strlen(msg2)));
  printf("%s\n", buf);
  close(fd);
  remove(path);
  return 0;
}
