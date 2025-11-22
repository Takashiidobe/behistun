#include <stdio.h>
#include <string.h>

int main(void) {
  char buf[32] = "abcdef";
  memmove(buf + 2, buf, 4); /* ab -> ababcd */
  buf[6] = '\0';
  printf("%s\n", buf);
  return 0;
}
