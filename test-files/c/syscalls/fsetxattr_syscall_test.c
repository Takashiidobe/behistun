#include <sys/syscall.h>
#include <unistd.h>

int main() {
    int fd = syscall(SYS_memfd_create, "fsetxattr_test", 0);
    if (fd < 0) {
        return 1;
    }
    long res = syscall(SYS_fsetxattr, fd, "user.test", "v", 1, 0);
    syscall(SYS_close, fd);
    return res == 0 || res < 0 ? 0 : 1;
}
