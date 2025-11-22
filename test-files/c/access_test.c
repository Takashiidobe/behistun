#include <fcntl.h>
#include <stdio.h>
#include <unistd.h>

int main(void) {
  // Create a test file
  int fd = open("/tmp/access_test.txt", O_CREAT | O_WRONLY, 0644);
  if (fd < 0) {
    perror("open");
    return 1;
  }
  write(fd, "test\n", 5);
  close(fd);

  // Test access - check if file exists
  if (access("/tmp/access_test.txt", F_OK) == 0) {
    printf("file exists\n");
  }

  // Check read permission
  if (access("/tmp/access_test.txt", R_OK) == 0) {
    printf("file readable\n");
  }

  // Check write permission
  if (access("/tmp/access_test.txt", W_OK) == 0) {
    printf("file writable\n");
  }

  // Test non-existent file
  if (access("/tmp/nonexistent_file_12345.txt", F_OK) != 0) {
    printf("nonexistent file not found\n");
  }

  unlink("/tmp/access_test.txt");

  return 0;
}
