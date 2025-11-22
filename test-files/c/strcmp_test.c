#include <stdio.h>
#include <string.h>

int main(void) {
  printf("%d\n", strcmp("abc", "abc"));
  printf("%d\n", strcmp("abc", "xyz") < 0 ? -1 : 1);
  printf("%d\n", strcmp("xyz", "abc") > 0 ? 1 : -1);
  printf("%d\n", strcmp("", ""));
  printf("%d\n", strcmp("a", "") > 0 ? 1 : -1);

  return 0;
}
