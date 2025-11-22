#include <sys/statfs.h>
#include <sys/syscall.h>
#include <unistd.h>

#ifndef SYS_statfs64
#define SYS_statfs64 SYS_statfs
#endif

int main() {
    struct statfs st;
    long res = syscall(SYS_statfs64, ".", &st);
    return res == 0 ? 0 : 1;
}
