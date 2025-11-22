#include <sys/syscall.h>
#include <unistd.h>

int main() { return syscall(SYS_getpid) > 0 ? 0 : 1; }
