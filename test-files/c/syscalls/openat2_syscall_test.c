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

// RESOLVE_* flags
#define RESOLVE_NO_XDEV 0x01
#define RESOLVE_NO_MAGICLINKS 0x02
#define RESOLVE_NO_SYMLINKS 0x04
#define RESOLVE_BENEATH 0x08
#define RESOLVE_IN_ROOT 0x10
#define RESOLVE_CACHED 0x20

int main() {
  // Test 1: Basic openat2 with O_RDONLY
  struct open_how how1;
  memset(&how1, 0, sizeof(how1));
  how1.flags = O_RDONLY;
  how1.mode = 0;
  how1.resolve = 0;

  int fd = syscall(SYS_openat2, AT_FDCWD, "/dev/null", &how1, sizeof(how1));
  if (fd < 0) {
    // openat2 not supported (kernel < 5.6) - this is OK
    if (errno == ENOSYS) {
      return 0; // Success - syscall not available
    }
    return 1; // Unexpected error
  }
  close(fd);

  // Test 2: Create a temporary file
  how1.flags = O_CREAT | O_RDWR | O_EXCL;
  how1.mode = 0600;
  how1.resolve = 0;

  fd = syscall(SYS_openat2, AT_FDCWD, "/tmp/openat2_test_file", &how1,
               sizeof(how1));
  if (fd < 0) {
    if (errno == ENOSYS) {
      return 0; // Success - syscall not available
    }
    return 2;
  }

  // Write some data
  const char *test_data = "openat2 test";
  if (write(fd, test_data, strlen(test_data)) != (ssize_t)strlen(test_data)) {
    close(fd);
    unlink("/tmp/openat2_test_file");
    return 3;
  }
  close(fd);

  // Test 3: Open existing file with openat2
  how1.flags = O_RDONLY;
  how1.mode = 0;
  how1.resolve = 0;

  fd = syscall(SYS_openat2, AT_FDCWD, "/tmp/openat2_test_file", &how1,
               sizeof(how1));
  if (fd < 0) {
    unlink("/tmp/openat2_test_file");
    return 4;
  }

  // Read and verify
  char buf[32];
  ssize_t nread = read(fd, buf, sizeof(buf));
  close(fd);

  if (nread != (ssize_t)strlen(test_data) ||
      memcmp(buf, test_data, strlen(test_data)) != 0) {
    unlink("/tmp/openat2_test_file");
    return 5;
  }

  // Clean up
  unlink("/tmp/openat2_test_file");

  // Test 4: Test with RESOLVE_NO_SYMLINKS (should work with non-symlink)
  how1.flags = O_RDONLY;
  how1.mode = 0;
  how1.resolve = RESOLVE_NO_SYMLINKS;

  fd = syscall(SYS_openat2, AT_FDCWD, "/dev/null", &how1, sizeof(how1));
  if (fd < 0) {
    if (errno == ENOSYS) {
      return 0; // Success - syscall not available
    }
    // Some systems may not support all resolve flags
    if (errno == EINVAL) {
      return 0; // OK
    }
    return 6;
  }
  close(fd);

  return 0; // Success
}
