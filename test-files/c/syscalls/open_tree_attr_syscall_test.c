#include <errno.h>
#include <fcntl.h>
#include <stdint.h>
#include <string.h>
#include <sys/mount.h>
#include <sys/syscall.h>
#include <unistd.h>

// AT_* flags
#ifndef AT_FDCWD
#define AT_FDCWD -100
#endif

#ifndef OPEN_TREE_CLONE
#define OPEN_TREE_CLONE 1
#endif

#ifndef OPEN_TREE_CLOEXEC
#define OPEN_TREE_CLOEXEC O_CLOEXEC
#endif

// Mount attribute flags
#ifndef MOUNT_ATTR_RDONLY
#define MOUNT_ATTR_RDONLY 0x00000001
#endif

#ifndef MOUNT_ATTR_NOSUID
#define MOUNT_ATTR_NOSUID 0x00000002
#endif

// struct mount_attr for open_tree_attr
struct mount_attr {
  uint64_t attr_set;
  uint64_t attr_clr;
  uint64_t propagation;
  uint64_t userns_fd;
};

int main() {
  // Test 1: Basic open_tree_attr with NULL attr (behaves like open_tree)
  int fd =
      syscall(SYS_open_tree_attr, AT_FDCWD, "/tmp", OPEN_TREE_CLONE, NULL, 0);

  if (fd < 0) {
    // open_tree_attr not supported - this is OK
    if (errno == ENOSYS) {
      return 0; // Success - syscall not available
    }
    // May also fail with EPERM if not privileged
    if (errno == EPERM) {
      return 0; // OK - need CAP_SYS_ADMIN for OPEN_TREE_CLONE
    }
    return 1; // Unexpected error
  }
  close(fd);

  // Test 2: open_tree_attr with mount attributes
  struct mount_attr attr;
  memset(&attr, 0, sizeof(attr));
  attr.attr_set = MOUNT_ATTR_RDONLY;
  attr.attr_clr = 0;
  attr.propagation = 0;
  attr.userns_fd = 0;

  fd = syscall(SYS_open_tree_attr, AT_FDCWD, "/tmp", OPEN_TREE_CLONE, &attr,
               sizeof(attr));

  if (fd < 0) {
    if (errno == ENOSYS) {
      return 0; // Success - syscall not available
    }
    if (errno == EPERM) {
      return 0; // OK - need privileges
    }
    return 2; // Unexpected error
  }
  close(fd);

  // Test 3: Test without OPEN_TREE_CLONE (just get O_PATH fd)
  memset(&attr, 0, sizeof(attr));
  fd = syscall(SYS_open_tree_attr, AT_FDCWD, "/tmp", 0, NULL, 0);

  if (fd < 0) {
    if (errno == ENOSYS) {
      return 0;
    }
    return 3;
  }
  close(fd);

  return 0; // Success
}
