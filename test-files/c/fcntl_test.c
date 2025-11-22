#include <fcntl.h>
#include <stdio.h>
#include <unistd.h>

int main(void) {
  // Create a test file
  int fd = open("/tmp/fcntl_test.txt", O_CREAT | O_RDWR, 0644);
  if (fd < 0) {
    perror("open");
    return 1;
  }

  // Test F_GETFL (get file status flags)
  int flags = fcntl(fd, F_GETFL);
  if (flags >= 0) {
    printf("F_GETFL works\n");
  }

  // Test F_SETFL (set file status flags)
  if (fcntl(fd, F_SETFL, flags | O_APPEND) >= 0) {
    printf("F_SETFL works\n");
  }

  // Test F_GETFD (get file descriptor flags)
  int fd_flags = fcntl(fd, F_GETFD);
  if (fd_flags >= 0) {
    printf("F_GETFD works\n");
  }

  // Test F_SETFD (set close-on-exec)
  if (fcntl(fd, F_SETFD, FD_CLOEXEC) >= 0) {
    printf("F_SETFD works\n");
  }

  close(fd);
  unlink("/tmp/fcntl_test.txt");

  return 0;
}
