#include <sys/syscall.h>
#include <unistd.h>

int main() { return syscall(SYS_geteuid) == geteuid() ? 0 : 1; }
