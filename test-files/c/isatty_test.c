#include <fcntl.h>
#include <stdio.h>
#include <unistd.h>

int main(void) {
  // Test isatty with stdin/stdout/stderr
  // Note: these may or may not be ttys depending on test environment
  int result = isatty(0);
  printf("stdin isatty: %d\n", result);

  result = isatty(1);
  printf("stdout isatty: %d\n", result);

  result = isatty(2);
  printf("stderr isatty: %d\n", result);

  // Test with a regular file (should return 0)
  int fd = open("/tmp/isatty_test.txt", O_CREAT | O_WRONLY, 0644);
  if (fd >= 0) {
    result = isatty(fd);
    if (result == 0) {
      printf("file is not tty\n");
    }
    close(fd);
    unlink("/tmp/isatty_test.txt");
  }

  return 0;
}
