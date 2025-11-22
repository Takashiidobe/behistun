#include <sys/syscall.h>
#include <unistd.h>

int main() {
    const char *path = "syscall_listxattr_test.txt";
    int fd = syscall(SYS_creat, path, 0644);
    if (fd < 0) {
        return 1;
    }
    syscall(SYS_close, fd);

    char buf[16];
    long res = syscall(SYS_listxattr, path, buf, sizeof(buf));
    syscall(SYS_unlink, path);
    return res >= 0 || res < 0 ? 0 : 1;
}
