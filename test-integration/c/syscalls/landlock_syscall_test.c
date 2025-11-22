#include <sys/syscall.h>
#include <unistd.h>
#include <errno.h>
#include <fcntl.h>
#include <stdint.h>
#include <sys/prctl.h>

// O_PATH may not be defined in older headers
#ifndef O_PATH
#define O_PATH 010000000
#endif

// PR_SET_NO_NEW_PRIVS might be missing on older libc headers
#ifndef PR_SET_NO_NEW_PRIVS
#define PR_SET_NO_NEW_PRIVS 38
#endif

// Landlock constants (from linux/landlock.h)
#define LANDLOCK_CREATE_RULESET_VERSION (1U << 0)

// Rule types
#define LANDLOCK_RULE_PATH_BENEATH 1

// Access rights for files
#define LANDLOCK_ACCESS_FS_EXECUTE (1ULL << 0)
#define LANDLOCK_ACCESS_FS_WRITE_FILE (1ULL << 1)
#define LANDLOCK_ACCESS_FS_READ_FILE (1ULL << 2)
#define LANDLOCK_ACCESS_FS_READ_DIR (1ULL << 3)

// Structures (must match kernel)
struct landlock_ruleset_attr {
    uint64_t handled_access_fs;
};

struct landlock_path_beneath_attr {
    uint64_t allowed_access;
    int32_t parent_fd;
};

int main() {
    // Test 1: Create a ruleset
    struct landlock_ruleset_attr ruleset_attr = {
        .handled_access_fs = LANDLOCK_ACCESS_FS_READ_FILE |
                            LANDLOCK_ACCESS_FS_WRITE_FILE |
                            LANDLOCK_ACCESS_FS_READ_DIR,
    };

    int ruleset_fd = syscall(SYS_landlock_create_ruleset,
                             &ruleset_attr,
                             sizeof(ruleset_attr),
                             0);

    if (ruleset_fd == -1) {
        // Landlock not supported (ENOSYS) or disabled (EOPNOTSUPP)
        if (errno == ENOSYS || errno == EOPNOTSUPP) {
            // Expected on systems without Landlock
            return 0;
        }
        // Unexpected error
        return 1;
    }

    // Test 2: Add a rule allowing access to /tmp
    int path_fd = open("/tmp", O_PATH | O_CLOEXEC);
    if (path_fd >= 0) {
        struct landlock_path_beneath_attr path_attr = {
            .allowed_access = LANDLOCK_ACCESS_FS_READ_FILE |
                             LANDLOCK_ACCESS_FS_WRITE_FILE |
                             LANDLOCK_ACCESS_FS_READ_DIR,
            .parent_fd = path_fd,
        };

        int add_result = syscall(SYS_landlock_add_rule,
                                 ruleset_fd,
                                 LANDLOCK_RULE_PATH_BENEATH,
                                 &path_attr,
                                 0);

        close(path_fd);

        if (add_result == -1 && errno != ENOSYS && errno != EOPNOTSUPP) {
            close(ruleset_fd);
            return 1;
        }
    }

    // Test 3: Restrict self
    // Landlock requires NO_NEW_PRIVS to be set.
    if (prctl(PR_SET_NO_NEW_PRIVS, 1, 0, 0, 0) == -1) {
        if (errno == ENOSYS) {
            close(ruleset_fd);
            return 0; // Old kernels without prctl flag
        }
        close(ruleset_fd);
        return 1;
    }

    int restrict_result = syscall(SYS_landlock_restrict_self, ruleset_fd, 0);

    close(ruleset_fd);

    if (restrict_result == -1) {
        if (errno == ENOSYS || errno == EOPNOTSUPP) {
            return 0; // Not supported
        }
        return 1; // Unexpected error
    }

    // Success! Landlock is working
    return 0;
}
