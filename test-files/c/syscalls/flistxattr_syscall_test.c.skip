#include <sys/syscall.h>
#include <unistd.h>

int main() {
    int fd = syscall(SYS_memfd_create, "flistxattr_test", 0);
    if (fd < 0) {
        return 1;
    }
    char buf[16];
    long res = syscall(SYS_flistxattr, fd, buf, sizeof(buf));
    syscall(SYS_close, fd);
    return res >= 0 || res < 0 ? 0 : 1;
}
