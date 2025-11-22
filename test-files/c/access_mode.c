#include <assert.h>
#include <stdio.h>
#include <unistd.h>

int main(void) {
  assert(access("Cargo.toml", R_OK) == 0);
  printf("ok\n");
  return 0;
}
