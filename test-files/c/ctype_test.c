#include <assert.h>
#include <ctype.h>
#include <stdio.h>

int main(void) {
  assert(isalpha('A'));
  assert(isupper('Z'));
  assert(islower('z'));
  assert(isdigit('5'));
  assert(isalnum('9'));
  assert(isspace(' '));
  assert(ispunct('!'));
  assert(isxdigit('f'));
  assert(tolower('Q') == 'q');
  assert(toupper('m') == 'M');
  printf("ctype ok\n");
  return 0;
}
