#include <errno.h>
#include <fcntl.h>
#include <stdio.h>
#include <string.h>
#include <sys/stat.h>
#include <sys/syscall.h>
#include <unistd.h>

// AT_* flags
#ifndef AT_FDCWD
#define AT_FDCWD -100
#endif

#ifndef AT_SYMLINK_NOFOLLOW
#define AT_SYMLINK_NOFOLLOW 0x100
#endif

int main() {
  const char *test_file = "/tmp/fchmodat2_test_file";
  const char *test_symlink = "/tmp/fchmodat2_test_symlink";

  // Clean up any previous test files
  unlink(test_symlink);
  unlink(test_file);

  // Test 1: Create a test file
  int fd = open(test_file, O_CREAT | O_RDWR | O_EXCL, 0600);
  if (fd < 0) {
    return 1;
  }
  close(fd);

  // Test 2: Change mode using fchmodat2
  int result = syscall(SYS_fchmodat2, AT_FDCWD, test_file, 0644, 0);

  if (result < 0) {
    // fchmodat2 not supported (kernel < 6.6) - this is OK
    if (errno == ENOSYS) {
      unlink(test_file);
      return 0; // Success - syscall not available
    }
    unlink(test_file);
    return 2; // Unexpected error
  }

  // Verify the mode was changed
  struct stat st;
  if (stat(test_file, &st) < 0) {
    unlink(test_file);
    return 3;
  }

  if ((st.st_mode & 0777) != 0644) {
    unlink(test_file);
    return 4;
  }

  // Test 3: Create a symlink to the test file
  if (symlink(test_file, test_symlink) < 0) {
    unlink(test_file);
    return 5;
  }

  // Test 4: Change mode of symlink itself using AT_SYMLINK_NOFOLLOW
  // This is the key feature of fchmodat2 - it actually respects this flag
  result =
      syscall(SYS_fchmodat2, AT_FDCWD, test_symlink, 0755, AT_SYMLINK_NOFOLLOW);

  if (result < 0) {
    if (errno == ENOSYS) {
      unlink(test_symlink);
      unlink(test_file);
      return 0; // Success - syscall not available
    }
    // ENOTSUP is OK - changing symlink permissions may not be supported on all
    // filesystems
    if (errno == EOPNOTSUPP || errno == ENOTSUP) {
      unlink(test_symlink);
      unlink(test_file);
      return 0;
    }
    unlink(test_symlink);
    unlink(test_file);
    return 6;
  }

  // Test 5: Change mode without AT_SYMLINK_NOFOLLOW (should affect target)
  result = syscall(SYS_fchmodat2, AT_FDCWD, test_symlink, 0640, 0);

  if (result < 0) {
    if (errno == ENOSYS) {
      unlink(test_symlink);
      unlink(test_file);
      return 0;
    }
    unlink(test_symlink);
    unlink(test_file);
    return 7;
  }

  // Verify the target file's mode was changed
  if (stat(test_file, &st) < 0) {
    unlink(test_symlink);
    unlink(test_file);
    return 8;
  }

  if ((st.st_mode & 0777) != 0640) {
    unlink(test_symlink);
    unlink(test_file);
    return 9;
  }

  // Clean up
  unlink(test_symlink);
  unlink(test_file);

  return 0; // Success
}
