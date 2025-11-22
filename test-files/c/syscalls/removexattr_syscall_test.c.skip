#include <sys/syscall.h>
#include <unistd.h>

int main() {
    long res = syscall(SYS_removexattr, "nonexistent", "user.test");
    (void)res;
    return 0;
}
