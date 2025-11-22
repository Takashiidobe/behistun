#include <sys/syscall.h>
#include <unistd.h>

int main() { return syscall(SYS_getgid) == getgid() ? 0 : 1; }
