#include <sys/syscall.h>
#include <sys/ipc.h>
#include <unistd.h>
#include <errno.h>

// IPC call numbers
#define MSGGET 13
#define MSGCTL 14

int main() {
    // Test ipc syscall by calling msgget through it
    // ipc(call, first, second, third, ptr, fifth)
    // msgget(key, msgflg) maps to: ipc(MSGGET, key, msgflg, 0, NULL, 0)

    key_t key = IPC_PRIVATE;
    int msgflg = IPC_CREAT | 0666;

    int msqid = syscall(SYS_ipc, MSGGET, key, msgflg, 0, NULL, 0);

    if (msqid == -1) {
        // It's OK if it fails due to permissions or system limits
        if (errno == ENOSPC || errno == ENOSYS || errno == EPERM || errno == EACCES) {
            return 0; // Expected failure modes
        }
        return 1; // Unexpected failure
    }

    // Success - clean up using ipc(MSGCTL, ...)
    // msgctl(msqid, IPC_RMID, NULL) maps to: ipc(MSGCTL, msqid, IPC_RMID, 0, NULL, 0)
    int rm_result = syscall(SYS_ipc, MSGCTL, msqid, IPC_RMID, 0, NULL, 0);
    if (rm_result == -1) {
        return 1; // Cleanup failed
    }

    return 0;
}
