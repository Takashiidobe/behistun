#include <sys/syscall.h>
#include <unistd.h>
#include <fcntl.h>
#include <sys/stat.h>
#include <errno.h>
#include <stdio.h>
#include <string.h>

#ifndef AT_FDCWD
#define AT_FDCWD -100
#endif

#ifndef AT_SYMLINK_NOFOLLOW
#define AT_SYMLINK_NOFOLLOW 0x100
#endif

int main() {
    const char *test_file = "/tmp/fchmodat2_test_file";

    printf("Testing fchmodat2 syscall...\n");

    // Clean up
    unlink(test_file);

    // Create a test file
    printf("Creating test file...\n");
    int fd = open(test_file, O_CREAT | O_RDWR | O_EXCL, 0600);
    if (fd < 0) {
        printf("Failed to create test file: errno=%d\n", errno);
        return 1;
    }
    close(fd);

    // Check initial mode
    struct stat st;
    if (stat(test_file, &st) < 0) {
        printf("Failed to stat file: errno=%d\n", errno);
        unlink(test_file);
        return 1;
    }
    printf("Initial mode: 0%o\n", st.st_mode & 0777);

    // Change mode using fchmodat2
    printf("Calling fchmodat2 to change mode to 0644...\n");
    int result = syscall(SYS_fchmodat2, AT_FDCWD, test_file, 0644, 0);
    if (result < 0) {
        printf("Result: %d, errno=%d\n", result, errno);
        if (errno == ENOSYS) {
            printf("ENOSYS - fchmodat2 not supported by kernel\n");
            unlink(test_file);
            return 0;
        }
        printf("Unexpected error\n");
        unlink(test_file);
        return 2;
    }
    printf("Result: %d (success)\n", result);

    // Verify the mode was changed
    if (stat(test_file, &st) < 0) {
        printf("Failed to stat file after chmod: errno=%d\n", errno);
        unlink(test_file);
        return 3;
    }
    printf("New mode: 0%o\n", st.st_mode & 0777);

    if ((st.st_mode & 0777) != 0644) {
        printf("Mode mismatch! Expected 0644, got 0%o\n", st.st_mode & 0777);
        unlink(test_file);
        return 4;
    }

    printf("Mode changed successfully!\n");

    // Test with relative dirfd
    printf("\nTesting with dirfd...\n");
    int dirfd = open("/tmp", O_RDONLY | O_DIRECTORY);
    if (dirfd < 0) {
        printf("Failed to open /tmp: errno=%d\n", errno);
        unlink(test_file);
        return 5;
    }

    result = syscall(SYS_fchmodat2, dirfd, "fchmodat2_test_file", 0600, 0);
    close(dirfd);

    if (result < 0) {
        printf("Result: %d, errno=%d\n", result, errno);
        if (errno == ENOSYS) {
            printf("ENOSYS - fchmodat2 not supported\n");
            unlink(test_file);
            return 0;
        }
        printf("Unexpected error with dirfd\n");
        unlink(test_file);
        return 6;
    }
    printf("Result: %d (success)\n", result);

    // Verify
    if (stat(test_file, &st) < 0) {
        unlink(test_file);
        return 7;
    }
    printf("Mode after dirfd test: 0%o\n", st.st_mode & 0777);

    if ((st.st_mode & 0777) != 0600) {
        printf("Mode mismatch!\n");
        unlink(test_file);
        return 8;
    }

    printf("Dirfd test passed!\n");

    // Clean up
    unlink(test_file);

    printf("\nAll tests passed!\n");
    return 0;
}
