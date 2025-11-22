#include <stdio.h>
#include <string.h>

int main(void) {
  const char *s1 = "hello";
  const char *s2 = "world!";
  const char *s3 = "";

  printf("%zu\n", strlen(s1));
  printf("%zu\n", strlen(s2));
  printf("%zu\n", strlen(s3));

  return 0;
}
