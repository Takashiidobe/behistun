#include <sys/syscall.h>
#include <unistd.h>

int main() {
    int fd = syscall(SYS_memfd_create, "fgetxattr_test", 0);
    if (fd < 0) {
        return 1;
    }
    char buf[4];
    long res = syscall(SYS_fgetxattr, fd, "user.test", buf, sizeof(buf));
    syscall(SYS_close, fd);
    return res == 0 || res < 0 ? 0 : 1;
}
