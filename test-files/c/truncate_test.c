#include <fcntl.h>
#include <stdio.h>
#include <sys/stat.h>
#include <unistd.h>

int main(void) {
  // Create a test file with some content
  int fd = open("/tmp/truncate_test.txt", O_CREAT | O_WRONLY, 0644);
  if (fd < 0) {
    perror("open");
    return 1;
  }
  write(fd, "Hello, World! This is a test.\n", 31);
  close(fd);

  // Get original size
  struct stat st;
  stat("/tmp/truncate_test.txt", &st);
  if (st.st_size == 31) {
    printf("original size ok\n");
  }

  // Truncate to smaller size
  if (truncate("/tmp/truncate_test.txt", 10) != 0) {
    perror("truncate");
    unlink("/tmp/truncate_test.txt");
    return 1;
  }

  printf("truncate works\n");

  // Verify new size
  stat("/tmp/truncate_test.txt", &st);
  if (st.st_size == 10) {
    printf("truncated size ok\n");
  }

  // Test ftruncate
  fd = open("/tmp/truncate_test.txt", O_WRONLY);
  if (fd >= 0) {
    if (ftruncate(fd, 5) == 0) {
      printf("ftruncate works\n");
    }
    close(fd);

    stat("/tmp/truncate_test.txt", &st);
    if (st.st_size == 5) {
      printf("ftruncated size ok\n");
    }
  }

  unlink("/tmp/truncate_test.txt");

  return 0;
}
