#include <sys/select.h>
#include <sys/syscall.h>
#include <unistd.h>

int main() {
    fd_set set;
    FD_ZERO(&set);
    struct timeval tv = {0, 0};
    return syscall(SYS_select, 0, &set, 0, 0, &tv) >= 0 ? 0 : 1;
}
