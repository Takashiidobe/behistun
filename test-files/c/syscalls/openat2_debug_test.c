#include <errno.h>
#include <fcntl.h>
#include <stdint.h>
#include <stdio.h>
#include <string.h>
#include <sys/syscall.h>
#include <unistd.h>

// struct open_how for openat2 syscall
struct open_how {
  uint64_t flags;
  uint64_t mode;
  uint64_t resolve;
};

int main() {
  printf("Testing openat2 syscall...\n");

  // Test 1: Basic openat2 with O_RDONLY
  struct open_how how1;
  memset(&how1, 0, sizeof(how1));
  how1.flags = O_RDONLY;
  how1.mode = 0;
  how1.resolve = 0;

  printf("Opening /dev/null with openat2...\n");
  int fd = syscall(SYS_openat2, AT_FDCWD, "/dev/null", &how1, sizeof(how1));
  printf("Result: fd=%d, errno=%d\n", fd, errno);

  if (fd < 0) {
    if (errno == ENOSYS) {
      printf("ENOSYS - openat2 not supported by kernel\n");
      return 0;
    }
    printf("Unexpected error opening /dev/null\n");
    return 1;
  }
  printf("Successfully opened /dev/null, fd=%d\n", fd);
  close(fd);

  // Test 2: Create a temporary file
  printf("\nCreating /tmp/openat2_test_file...\n");
  how1.flags = O_CREAT | O_RDWR | O_EXCL;
  how1.mode = 0600;
  how1.resolve = 0;

  fd = syscall(SYS_openat2, AT_FDCWD, "/tmp/openat2_test_file", &how1,
               sizeof(how1));
  printf("Result: fd=%d, errno=%d\n", fd, errno);

  if (fd < 0) {
    if (errno == ENOSYS) {
      printf("ENOSYS - openat2 not supported\n");
      return 0;
    }
    printf("Failed to create file\n");
    return 2;
  }

  // Write some data
  const char *test_data = "openat2 test";
  printf("Writing test data...\n");
  ssize_t nwritten = write(fd, test_data, strlen(test_data));
  printf("Wrote %ld bytes\n", (long)nwritten);

  if (nwritten != (ssize_t)strlen(test_data)) {
    close(fd);
    unlink("/tmp/openat2_test_file");
    return 3;
  }
  close(fd);

  // Test 3: Open existing file
  printf("\nReopening file for reading...\n");
  how1.flags = O_RDONLY;
  how1.mode = 0;
  how1.resolve = 0;

  fd = syscall(SYS_openat2, AT_FDCWD, "/tmp/openat2_test_file", &how1,
               sizeof(how1));
  printf("Result: fd=%d, errno=%d\n", fd, errno);

  if (fd < 0) {
    unlink("/tmp/openat2_test_file");
    return 4;
  }

  // Read and verify
  char buf[32];
  ssize_t nread = read(fd, buf, sizeof(buf));
  printf("Read %ld bytes\n", (long)nread);
  close(fd);

  if (nread != (ssize_t)strlen(test_data)) {
    printf("Read wrong number of bytes\n");
    unlink("/tmp/openat2_test_file");
    return 5;
  }

  if (memcmp(buf, test_data, strlen(test_data)) != 0) {
    printf("Data mismatch\n");
    unlink("/tmp/openat2_test_file");
    return 5;
  }

  printf("Data verified successfully!\n");

  // Clean up
  unlink("/tmp/openat2_test_file");

  printf("\nAll tests passed!\n");
  return 0;
}
