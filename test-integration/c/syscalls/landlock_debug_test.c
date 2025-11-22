#include <sys/syscall.h>
#include <unistd.h>
#include <errno.h>
#include <fcntl.h>
#include <stdint.h>
#include <stdio.h>
#include <sys/prctl.h>

// O_PATH may not be defined in older headers
#ifndef O_PATH
#define O_PATH 010000000
#endif

// PR_SET_NO_NEW_PRIVS might be missing on older libc headers
#ifndef PR_SET_NO_NEW_PRIVS
#define PR_SET_NO_NEW_PRIVS 38
#endif

// Landlock constants
struct landlock_ruleset_attr {
    uint64_t handled_access_fs;
};

int main() {
    printf("Testing landlock_create_ruleset...\n");

    struct landlock_ruleset_attr ruleset_attr = {
        .handled_access_fs = (1ULL << 2),  // LANDLOCK_ACCESS_FS_READ_FILE
    };

    int ruleset_fd = syscall(SYS_landlock_create_ruleset,
                             &ruleset_attr,
                             sizeof(ruleset_attr),
                             0);

    printf("landlock_create_ruleset returned: %d, errno: %d\n", ruleset_fd, errno);

    if (ruleset_fd == -1) {
        if (errno == ENOSYS) {
            printf("ENOSYS - not implemented\n");
            return 0;
        }
        if (errno == EOPNOTSUPP) {
            printf("EOPNOTSUPP - not supported\n");
            return 0;
        }
        printf("Unexpected error: %d\n", errno);
        return 1;
    }

    printf("Got ruleset_fd: %d\n", ruleset_fd);

    // Test restrict_self
    printf("Setting NO_NEW_PRIVS...\n");
    if (prctl(PR_SET_NO_NEW_PRIVS, 1, 0, 0, 0) == -1) {
        if (errno == ENOSYS) {
            printf("PR_SET_NO_NEW_PRIVS not supported, skipping restrict_self\n");
            close(ruleset_fd);
            return 0;
        }
        printf("prctl(NO_NEW_PRIVS) failed, errno: %d\n", errno);
        close(ruleset_fd);
        return 1;
    }

    printf("Testing landlock_restrict_self...\n");
    int restrict_result = syscall(SYS_landlock_restrict_self, ruleset_fd, 0);
    printf("landlock_restrict_self returned: %d, errno: %d\n", restrict_result, errno);

    close(ruleset_fd);

    if (restrict_result == -1) {
        if (errno == ENOSYS || errno == EOPNOTSUPP) {
            return 0;
        }
        printf("restrict_self failed with errno: %d\n", errno);
        return 1;
    }

    printf("Success!\n");
    return 0;
}
