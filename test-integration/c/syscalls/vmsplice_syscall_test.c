#include <sys/syscall.h>
#include <sys/uio.h>
#include <unistd.h>
#include <errno.h>
#include <string.h>

int main() {
    // Create a pipe
    int pipefd[2];
    if (pipe(pipefd) == -1) {
        return 1;
    }

    // Prepare test data in iovec
    const char *test_data = "Hello, vmsplice!";
    struct iovec iov;
    iov.iov_base = (void *)test_data;
    iov.iov_len = strlen(test_data);

    // Use vmsplice to write user memory to pipe
    // vmsplice(fd, iov, nr_segs, flags)
    ssize_t vmspliced = syscall(SYS_vmsplice, pipefd[1], &iov, 1, 0);

    if (vmspliced == -1) {
        if (errno == ENOSYS) {
            // vmsplice not implemented, that's okay
            close(pipefd[0]);
            close(pipefd[1]);
            return 0;
        }
        close(pipefd[0]);
        close(pipefd[1]);
        return 1;
    }

    // Verify we vmspliced the right amount
    if (vmspliced != strlen(test_data)) {
        close(pipefd[0]);
        close(pipefd[1]);
        return 1;
    }

    // Read from pipe to verify data
    char buffer[256];
    ssize_t bytes_read = read(pipefd[0], buffer, sizeof(buffer));

    close(pipefd[0]);
    close(pipefd[1]);

    if (bytes_read != strlen(test_data)) {
        return 1;
    }

    if (memcmp(buffer, test_data, strlen(test_data)) != 0) {
        return 1;
    }

    return 0;
}
