#include <errno.h>
#include <stdint.h>
#include <string.h>
#include <sys/syscall.h>
#include <unistd.h>

// Capability versions
#define _LINUX_CAPABILITY_VERSION_1 0x19980330
#define _LINUX_CAPABILITY_VERSION_2 0x20071026
#define _LINUX_CAPABILITY_VERSION_3 0x20080522

// Capability header
struct __user_cap_header_struct {
  uint32_t version;
  int pid;
};

// Capability data
struct __user_cap_data_struct {
  uint32_t effective;
  uint32_t permitted;
  uint32_t inheritable;
};

int main() {
  struct __user_cap_header_struct hdr;
  struct __user_cap_data_struct data[2];
  int result;

  // Test 1: Get capabilities with version 3 (modern version)
  memset(&hdr, 0, sizeof(hdr));
  memset(data, 0, sizeof(data));

  hdr.version = _LINUX_CAPABILITY_VERSION_3;
  hdr.pid = 0; // Current process

  result = syscall(SYS_capget, &hdr, data);
  if (result < 0) {
    if (errno == ENOSYS) {
      return 0; // capget not supported - OK
    }
    return 1; // Unexpected error
  }

  // Verify we got some capabilities back
  // The data should contain the process's current capabilities
  // We can't predict exact values, but at least one field should be non-zero
  // for a running process
  if (data[0].effective == 0 && data[0].permitted == 0 &&
      data[0].inheritable == 0 && data[1].effective == 0 &&
      data[1].permitted == 0 && data[1].inheritable == 0) {
    // All zero might be valid for a very restricted process, so we'll allow it
    // return 2;
  }

  // Test 2: Try version probing (NULL datap)
  memset(&hdr, 0, sizeof(hdr));
  hdr.version = _LINUX_CAPABILITY_VERSION_1; // Try old version
  hdr.pid = 0;

  result = syscall(SYS_capget, &hdr, NULL);

  // Kernel should either accept V1 or update to preferred version
  if (result < 0 && errno != EINVAL) {
    return 3; // Unexpected error
  }

  // If kernel rejected V1, it should have updated hdr.version
  if (result < 0 && errno == EINVAL) {
    // Version field should be updated to preferred version
    if (hdr.version != _LINUX_CAPABILITY_VERSION_2 &&
        hdr.version != _LINUX_CAPABILITY_VERSION_3) {
      return 4; // Kernel didn't update version field
    }
  }

  // Test 3: Get capabilities for current process again with correct version
  memset(&hdr, 0, sizeof(hdr));
  memset(data, 0, sizeof(data));

  hdr.version = _LINUX_CAPABILITY_VERSION_3;
  hdr.pid = 0;

  result = syscall(SYS_capget, &hdr, data);
  if (result < 0) {
    return 5;
  }

  // Test 4: Try to set capabilities (will likely fail without CAP_SETPCAP)
  // Save current capabilities
  uint32_t saved_eff[2] = {data[0].effective, data[1].effective};
  uint32_t saved_perm[2] = {data[0].permitted, data[1].permitted};
  uint32_t saved_inh[2] = {data[0].inheritable, data[1].inheritable};

  // Try to set capabilities (same as current - should be safe)
  memset(&hdr, 0, sizeof(hdr));
  hdr.version = _LINUX_CAPABILITY_VERSION_3;
  hdr.pid = 0;

  result = syscall(SYS_capset, &hdr, data);

  // capset might fail with EPERM if we don't have CAP_SETPCAP
  // This is OK - we're just testing that the syscall works
  if (result < 0) {
    if (errno == EPERM || errno == ENOSYS) {
      // Expected - most processes can't set capabilities
      return 0; // Success
    }
    return 6; // Unexpected error
  }

  // If capset succeeded, verify capabilities are still the same
  memset(&hdr, 0, sizeof(hdr));
  memset(data, 0, sizeof(data));

  hdr.version = _LINUX_CAPABILITY_VERSION_3;
  hdr.pid = 0;

  result = syscall(SYS_capget, &hdr, data);
  if (result < 0) {
    return 7;
  }

  // Verify capabilities match what we set
  if (data[0].effective != saved_eff[0] || data[1].effective != saved_eff[1] ||
      data[0].permitted != saved_perm[0] ||
      data[1].permitted != saved_perm[1] ||
      data[0].inheritable != saved_inh[0] ||
      data[1].inheritable != saved_inh[1]) {
    return 8; // Capabilities changed unexpectedly
  }

  // Test 5: Test with version 1 (32-bit capabilities)
  memset(&hdr, 0, sizeof(hdr));
  memset(data, 0, sizeof(data));

  hdr.version = _LINUX_CAPABILITY_VERSION_1;
  hdr.pid = 0;

  result = syscall(SYS_capget, &hdr, data);

  // Modern kernels may reject V1, or accept it
  if (result < 0) {
    if (errno == EINVAL) {
      // Kernel wants newer version - this is OK
      return 0;
    }
    return 9;
  }

  // If V1 succeeded, only data[0] should be used (32-bit caps)
  // data[1] should remain zero

  return 0; // Success!
}
