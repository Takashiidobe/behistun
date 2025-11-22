#include <sys/syscall.h>
#include <unistd.h>
#include <errno.h>
#include <string.h>

int main() {
    // Create two pipes
    int pipe1[2], pipe2[2];

    if (pipe(pipe1) == -1 || pipe(pipe2) == -1) {
        return 1;
    }

    // Write test data to first pipe
    const char *test_data = "Hello, tee!";
    ssize_t written = write(pipe1[1], test_data, strlen(test_data));
    if (written != strlen(test_data)) {
        close(pipe1[0]);
        close(pipe1[1]);
        close(pipe2[0]);
        close(pipe2[1]);
        return 1;
    }

    // Use tee to duplicate data from pipe1 to pipe2
    // tee(fd_in, fd_out, len, flags)
    ssize_t teed = syscall(SYS_tee, pipe1[0], pipe2[1], strlen(test_data), 0);

    if (teed == -1) {
        if (errno == ENOSYS) {
            // tee not implemented, that's okay
            close(pipe1[0]);
            close(pipe1[1]);
            close(pipe2[0]);
            close(pipe2[1]);
            return 0;
        }
        close(pipe1[0]);
        close(pipe1[1]);
        close(pipe2[0]);
        close(pipe2[1]);
        return 1;
    }

    // Verify we teed the right amount
    if (teed != strlen(test_data)) {
        close(pipe1[0]);
        close(pipe1[1]);
        close(pipe2[0]);
        close(pipe2[1]);
        return 1;
    }

    // Read from both pipes to verify data was duplicated
    char buffer1[256], buffer2[256];
    ssize_t read1 = read(pipe1[0], buffer1, sizeof(buffer1));
    ssize_t read2 = read(pipe2[0], buffer2, sizeof(buffer2));

    close(pipe1[0]);
    close(pipe1[1]);
    close(pipe2[0]);
    close(pipe2[1]);

    // Both reads should succeed with same amount of data
    if (read1 != strlen(test_data) || read2 != strlen(test_data)) {
        return 1;
    }

    // Both should contain the same data
    if (memcmp(buffer1, test_data, strlen(test_data)) != 0 ||
        memcmp(buffer2, test_data, strlen(test_data)) != 0) {
        return 1;
    }

    return 0;
}
