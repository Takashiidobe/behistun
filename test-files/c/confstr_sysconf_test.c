#include <stdio.h>
#include <unistd.h>

int main(void) {
  long clk = sysconf(_SC_CLK_TCK);
  long pagesz = sysconf(_SC_PAGESIZE);
  char buf[256];
  size_t len = confstr(_CS_PATH, buf, sizeof(buf));

  printf("clk=%ld pagesz=%ld path_len=%zu path=%s\n", clk, pagesz, len,
         (len > 0 && len < sizeof(buf)) ? buf : "(truncated)");
  return (clk > 0 && pagesz > 0) ? 0 : 1;
}
