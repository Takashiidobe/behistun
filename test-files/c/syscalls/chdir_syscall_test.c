#include <sys/syscall.h>
#include <unistd.h>

int main() { return syscall(SYS_chdir, ".") == 0 ? 0 : 1; }
