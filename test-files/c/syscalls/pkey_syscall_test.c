#include <errno.h>
#include <string.h>
#include <sys/mman.h>
#include <sys/syscall.h>
#include <unistd.h>

// Protection key access rights
#ifndef PKEY_DISABLE_ACCESS
#define PKEY_DISABLE_ACCESS 0x1
#endif

#ifndef PKEY_DISABLE_WRITE
#define PKEY_DISABLE_WRITE 0x2
#endif

int main() {
  // Test 1: Allocate a protection key
  int pkey = syscall(SYS_pkey_alloc, 0, 0);

  if (pkey < 0) {
    // If ENOSYS, pkey not supported - that's OK
    if (errno == ENOSYS) {
      return 0;
    }
    return 1; // Unexpected error
  }

  // pkey should be a valid key (typically 1-15 on x86_64)
  if (pkey < 0) {
    return 2;
  }

  // Test 2: Allocate memory with mmap
  void *addr = mmap(NULL, 4096, PROT_READ | PROT_WRITE,
                    MAP_PRIVATE | MAP_ANONYMOUS, -1, 0);

  if (addr == MAP_FAILED) {
    syscall(SYS_pkey_free, pkey);
    return 3;
  }

  // Test 3: Apply protection key to the memory
  int result =
      syscall(SYS_pkey_mprotect, addr, 4096, PROT_READ | PROT_WRITE, pkey);

  if (result < 0) {
    if (errno == ENOSYS) {
      munmap(addr, 4096);
      syscall(SYS_pkey_free, pkey);
      return 0;
    }
    munmap(addr, 4096);
    syscall(SYS_pkey_free, pkey);
    return 4;
  }

  // Test 4: Write to the memory (should work)
  char *ptr = (char *)addr;
  ptr[0] = 'A';
  ptr[100] = 'B';

  // Verify writes
  if (ptr[0] != 'A' || ptr[100] != 'B') {
    munmap(addr, 4096);
    syscall(SYS_pkey_free, pkey);
    return 5;
  }

  // Test 5: Test pkey_mprotect with different flags
  result = syscall(SYS_pkey_mprotect, addr, 4096, PROT_READ, pkey);

  if (result < 0) {
    munmap(addr, 4096);
    syscall(SYS_pkey_free, pkey);
    return 6;
  }

  // Test 6: Allocate another key with access restrictions
  int pkey2 = syscall(SYS_pkey_alloc, 0, PKEY_DISABLE_WRITE);

  if (pkey2 < 0) {
    munmap(addr, 4096);
    syscall(SYS_pkey_free, pkey);
    return 7;
  }

  // Test 7: Free the first key
  result = syscall(SYS_pkey_free, pkey);

  if (result < 0) {
    munmap(addr, 4096);
    syscall(SYS_pkey_free, pkey2);
    return 8;
  }

  // Test 8: Free the second key
  result = syscall(SYS_pkey_free, pkey2);

  if (result < 0) {
    munmap(addr, 4096);
    return 9;
  }

  // Clean up
  munmap(addr, 4096);

  return 0; // Success
}
