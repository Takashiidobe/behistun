#include <sys/syscall.h>
#include <unistd.h>

int main() {
    const char *target = "syscall_lgetxattr_target.txt";
    const char *linkp = "syscall_lgetxattr_link.txt";
    int fd = syscall(SYS_creat, target, 0644);
    if (fd < 0) {
        return 1;
    }
    syscall(SYS_close, fd);
    syscall(SYS_symlink, target, linkp);

    char buf[4];
    long res = syscall(SYS_lgetxattr, linkp, "user.test", buf, sizeof(buf));
    syscall(SYS_unlink, linkp);
    syscall(SYS_unlink, target);
    return res == 0 || res < 0 ? 0 : 1;
}
