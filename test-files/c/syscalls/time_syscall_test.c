#include <sys/syscall.h>
#include <unistd.h>

int main() { return syscall(SYS_time, 0) >= 0 ? 0 : 1; }
