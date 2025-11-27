#include <sys/mman.h>
#include <sys/syscall.h>
#include <unistd.h>

int main() {
    void *p = mmap(0, 4096, PROT_READ, MAP_PRIVATE | MAP_ANONYMOUS, -1, 0);
    if (p == MAP_FAILED) {
        return 0;
    }
    unsigned char vec[1];
    long res = syscall(SYS_mincore, p, 4096, vec);
    syscall(SYS_munmap, p, 4096);
    return res == 0 || res < 0 ? 0 : 1;
}
