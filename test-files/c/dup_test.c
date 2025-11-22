#include <fcntl.h>
#include <stdio.h>
#include <string.h>
#include <unistd.h>

int main(void) {
  // Create a test file
  int fd = open("/tmp/dup_test.txt", O_CREAT | O_WRONLY | O_TRUNC, 0644);
  if (fd < 0) {
    perror("open");
    return 1;
  }

  // Duplicate the file descriptor
  int fd2 = dup(fd);
  if (fd2 < 0) {
    perror("dup");
    close(fd);
    return 1;
  }

  printf("dup works\n");

  // Write through original fd
  write(fd, "hello ", 6);

  // Write through duplicated fd
  write(fd2, "world\n", 6);

  close(fd);
  close(fd2);

  // Read back and verify
  fd = open("/tmp/dup_test.txt", O_RDONLY);
  char buf[64];
  int n = read(fd, buf, sizeof(buf));
  close(fd);

  if (n > 0 && strncmp(buf, "hello world", 11) == 0) {
    printf("content correct\n");
  }

  unlink("/tmp/dup_test.txt");

  return 0;
}
