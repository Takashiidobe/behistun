#include <linux/futex.h>
#include <sys/syscall.h>
#include <sys/time.h>
#include <unistd.h>
#include <stdio.h>
#include <stdint.h>
#include <errno.h>
#include <time.h>

int futex_wait(int *futex_addr, int val, struct timespec *timeout) {
    return syscall(SYS_futex, futex_addr, FUTEX_WAIT, val, timeout, NULL, 0);
}

int futex_wake(int *futex_addr, int nr_wake) {
    return syscall(SYS_futex, futex_addr, FUTEX_WAKE, nr_wake, NULL, NULL, 0);
}

int main() {
    int futex_var = 0;
    int result;

    printf("Testing futex syscall...\n");
    printf("Initial futex value: %d\n", futex_var);

    // Test 1: Wake on futex with no waiters (should return 0)
    result = futex_wake(&futex_var, 1);
    printf("futex_wake (no waiters) result: %d\n", result);
    if (result != 0) {
        printf("FAIL: Expected 0 waiters woken, got %d\n", result);
        return 1;
    }

    // Test 2: Try to wait with wrong value (should fail immediately with EAGAIN)
    futex_var = 42;
    result = futex_wait(&futex_var, 0, NULL);  // Expect 0, but it's 42
    if (result == -1 && errno == EAGAIN) {
        printf("futex_wait (wrong value) correctly returned EAGAIN\n");
    } else {
        printf("FAIL: Expected EAGAIN, got result=%d errno=%d\n", result, errno);
        return 1;
    }

    // Test 3: Try wait with very short timeout
    // Note: Zero timeout may return EAGAIN on some systems instead of ETIMEDOUT
    struct timespec timeout = {0, 1000};  // 1 microsecond
    futex_var = 100;
    result = futex_wait(&futex_var, 100, &timeout);
    if (result == -1 && (errno == ETIMEDOUT || errno == EAGAIN)) {
        printf("futex_wait (short timeout) correctly timed out (errno=%d)\n", errno);
    } else {
        printf("FAIL: Expected timeout error, got result=%d errno=%d\n", result, errno);
        return 1;
    }

    // Test 4: Wake again on futex with no waiters
    result = futex_wake(&futex_var, 5);
    printf("futex_wake (still no waiters) result: %d\n", result);
    if (result != 0) {
        printf("FAIL: Expected 0 waiters woken, got %d\n", result);
        return 1;
    }

    printf("All futex tests passed!\n");
    return 0;
}
