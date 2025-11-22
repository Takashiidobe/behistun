#include <sys/syscall.h>
#include <unistd.h>

int main() { return syscall(SYS_getuid) == getuid() ? 0 : 1; }
