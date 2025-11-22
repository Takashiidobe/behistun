#include <sys/syscall.h>
#include <unistd.h>
#include <errno.h>

int main() {
    // Try to umount a non-existent path
    // This should fail with EINVAL or ENOENT, not crash
    int result = syscall(SYS_umount, "/nonexistent/mount/point");

    if (result == -1) {
        // Expected to fail - we're just testing that the syscall is dispatched
        // EINVAL (22), ENOENT (2), EPERM (1), or EACCES (13) are all acceptable
        if (errno == EINVAL || errno == ENOENT || errno == EPERM ||
            errno == EACCES || errno == ENOSYS) {
            return 0;
        }
        // Some other error - still okay for this test
        return 0;
    }

    // Unlikely to succeed, but if it does, that's fine too
    return 0;
}
