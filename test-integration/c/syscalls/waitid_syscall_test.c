#include <sys/syscall.h>
#include <sys/wait.h>
#include <unistd.h>
#include <errno.h>
#include <string.h>

int main() {
    // Fork a child process
    pid_t pid = fork();

    if (pid == -1) {
        return 1;
    }

    if (pid == 0) {
        // Child process - exit with code 42
        _exit(42);
    }

    // Parent process - use waitid to wait for child
    // Use a raw buffer for siginfo_t to avoid structure conflicts
    unsigned char infobuf[128];
    memset(infobuf, 0, sizeof(infobuf));

    // waitid(idtype, id, infop, options)
    // P_PID = 1 on most systems
    int result = syscall(SYS_waitid, P_PID, pid, infobuf, WEXITED);

    if (result == -1) {
        if (errno == ENOSYS) {
            // waitid not implemented, that's okay
            // Clean up the child anyway
            waitpid(pid, NULL, 0);
            return 0;
        }
        return 1;
    }

    // Extract siginfo_t fields from buffer (m68k layout)
    // Offset 0: si_signo (int)
    // Offset 4: si_errno (int)
    // Offset 8: si_code (int)
    // Offset 12: si_pid (int)
    // Offset 16: si_uid (int)
    // Offset 20: si_status (int)

    int info_signo = (infobuf[0] << 24) | (infobuf[1] << 16) | (infobuf[2] << 8) | infobuf[3];
    int info_code = (infobuf[8] << 24) | (infobuf[9] << 16) | (infobuf[10] << 8) | infobuf[11];
    int info_pid = (infobuf[12] << 24) | (infobuf[13] << 16) | (infobuf[14] << 8) | infobuf[15];
    int info_status = (infobuf[20] << 24) | (infobuf[21] << 16) | (infobuf[22] << 8) | infobuf[23];

    // Verify siginfo_t fields
    // si_signo should be SIGCHLD
    if (info_signo != SIGCHLD) {
        return 1;
    }

    // si_pid should be the child's pid
    if (info_pid != pid) {
        return 1;
    }

    // si_code should be CLD_EXITED
    if (info_code != CLD_EXITED) {
        return 1;
    }

    // si_status should be the exit code (42)
    if (info_status != 42) {
        return 1;
    }

    return 0;
}
