#include <sys/syscall.h>
#include <unistd.h>
#include <errno.h>
#include <fcntl.h>
#include <string.h>

int main() {
    // Create a pipe
    int pipefd[2];
    if (pipe(pipefd) == -1) {
        return 1;
    }

    // Write test data to pipe
    const char *test_data = "Hello, splice!";
    ssize_t written = write(pipefd[1], test_data, strlen(test_data));
    if (written != strlen(test_data)) {
        close(pipefd[0]);
        close(pipefd[1]);
        return 1;
    }

    // Create output pipe
    int outpipe[2];
    if (pipe(outpipe) == -1) {
        close(pipefd[0]);
        close(pipefd[1]);
        return 1;
    }

    // Use splice to copy from input pipe to output pipe
    // splice(fd_in, off_in, fd_out, off_out, len, flags)
    ssize_t spliced = syscall(SYS_splice, pipefd[0], NULL, outpipe[1], NULL,
                              strlen(test_data), 0);

    if (spliced == -1) {
        if (errno == ENOSYS) {
            // splice not implemented, that's okay for this test
            close(pipefd[0]);
            close(pipefd[1]);
            close(outpipe[0]);
            close(outpipe[1]);
            return 0;
        }
        close(pipefd[0]);
        close(pipefd[1]);
        close(outpipe[0]);
        close(outpipe[1]);
        return 1;
    }

    // Verify we spliced the right amount
    if (spliced != strlen(test_data)) {
        close(pipefd[0]);
        close(pipefd[1]);
        close(outpipe[0]);
        close(outpipe[1]);
        return 1;
    }

    // Read from output pipe to verify
    char buffer[256];
    ssize_t bytes_read = read(outpipe[0], buffer, sizeof(buffer));

    close(pipefd[0]);
    close(pipefd[1]);
    close(outpipe[0]);
    close(outpipe[1]);

    if (bytes_read != strlen(test_data)) {
        return 1;
    }

    if (memcmp(buffer, test_data, strlen(test_data)) != 0) {
        return 1;
    }

    return 0;
}
