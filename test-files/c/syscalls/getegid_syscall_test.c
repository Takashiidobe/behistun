#include <sys/syscall.h>
#include <unistd.h>

int main() { return syscall(SYS_getegid) == getegid() ? 0 : 1; }
