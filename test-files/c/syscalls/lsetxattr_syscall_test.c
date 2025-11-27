#include <sys/syscall.h>
#include <unistd.h>

int main() {
    const char *target = "syscall_lsetxattr_target.txt";
    const char *linkp = "syscall_lsetxattr_link.txt";
    int fd = syscall(SYS_creat, target, 0644);
    if (fd < 0) {
        return 1;
    }
    syscall(SYS_close, fd);
    if (syscall(SYS_symlink, target, linkp) < 0) {
        syscall(SYS_unlink, target);
        return 1;
    }

    long res = syscall(SYS_lsetxattr, linkp, "user.test", "v", 1, 0);
    syscall(SYS_unlink, linkp);
    syscall(SYS_unlink, target);
    return res == 0 || res < 0 ? 0 : 1;
}
