#include <sys/syscall.h>
#include <unistd.h>

int main() {
    const char *path = "syscall_setxattr_test.txt";
    int fd = syscall(SYS_creat, path, 0644);
    if (fd < 0) {
        return 1;
    }
    syscall(SYS_close, fd);

    long res = syscall(SYS_setxattr, path, "user.test", "v", 1, 0);
    syscall(SYS_unlink, path);
    return res == 0 || res < 0 ? 0 : 1;
}
