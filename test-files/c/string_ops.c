#include <stdio.h>
#include <string.h>

int main(void) {
  char buf[32];
  strcpy(buf, "foo");
  strcat(buf, "bar");
  size_t len = strlen(buf);
  int cmp = strcmp(buf, "foobar");
  printf("%s %zu %d\n", buf, len, cmp);
  return (int)(len + cmp);
}
