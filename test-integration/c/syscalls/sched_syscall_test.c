#include <sched.h>
#include <unistd.h>
#include <sys/syscall.h>
#include <time.h>

#define SYS_sched_setparam 154
#define SYS_sched_getparam 155
#define SYS_sched_setscheduler 156
#define SYS_sched_rr_get_interval 161

int main() {
    struct sched_param param;
    struct timespec ts;
    pid_t pid = getpid();
    int result;

    // Test 1: Get current scheduling parameters
    result = syscall(SYS_sched_getparam, pid, &param);
    if (result != 0) {
        return 1;
    }

    // Save original priority
    int original_priority = param.sched_priority;

    // Test 2: Set scheduling parameters (keep same priority)
    param.sched_priority = original_priority;
    result = syscall(SYS_sched_setparam, pid, &param);
    if (result != 0) {
        return 2;
    }

    // Test 3: Verify the priority was set
    param.sched_priority = -1;
    result = syscall(SYS_sched_getparam, pid, &param);
    if (result != 0) {
        return 3;
    }
    if (param.sched_priority != original_priority) {
        return 4;
    }

    // Test 4: Get current scheduler (should be SCHED_OTHER for normal processes)
    int policy = sched_getscheduler(pid);
    if (policy < 0) {
        return 5;
    }

    // Test 5: Set scheduler (keep same policy and priority)
    param.sched_priority = original_priority;
    result = syscall(SYS_sched_setscheduler, pid, policy, &param);
    if (result < 0) {
        return 6;
    }

    // Test 6: sched_rr_get_interval (may fail if not SCHED_RR, that's ok)
    // Just test that it doesn't crash
    result = syscall(SYS_sched_rr_get_interval, pid, &ts);
    // Don't check result - it will fail for SCHED_OTHER but shouldn't crash

    // Test 7: Test with pid 0 (current process)
    param.sched_priority = -1;
    result = syscall(SYS_sched_getparam, 0, &param);
    if (result != 0) {
        return 7;
    }
    if (param.sched_priority != original_priority) {
        return 8;
    }

    return 0;  // Success
}
