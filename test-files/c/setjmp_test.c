#include <setjmp.h>
#include <stdio.h>

static jmp_buf env;
static int counter = 0;

void do_jump(void) {
  counter++;
  longjmp(env, counter);
}

int main(void) {
  int val = setjmp(env);

  if (val == 0) {
    // First time through
    printf("setjmp works\n");
    do_jump();
    // Should not reach here
    return 1;
  } else if (val == 1) {
    // After first longjmp
    printf("longjmp works\n");
    return 0;
  }

  return 1;
}
