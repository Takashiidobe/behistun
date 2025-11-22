#include <sys/syscall.h>
#include <unistd.h>

int main() {
    long res = syscall(SYS_lremovexattr, "nonexistent", "user.test");
    (void)res;
    return 0;
}
