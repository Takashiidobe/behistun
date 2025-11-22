#include <sys/syscall.h>
#include <unistd.h>

int main() {
    int fd = syscall(SYS_memfd_create, "pread64_test", 0);
    if (fd < 0) {
        return 1;
    }
    syscall(SYS_write, fd, "data", 4);
    char buf[4];
    long res = syscall(SYS_pread64, fd, buf, 4, 0);
    syscall(SYS_close, fd);
    return res == 4 ? 0 : 1;
}
