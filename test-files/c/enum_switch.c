#include <stdio.h>

enum Color { RED = 1, GREEN = 2, BLUE = 3, OTHER = 99 };

static const char *to_str(enum Color c) {
  switch (c) {
  case RED:
    return "red";
  case GREEN:
    return "green";
  case BLUE:
    return "blue";
  default:
    return "other";
  }
}

int main(void) {
  enum Color vals[] = {RED, BLUE, OTHER, GREEN};
  for (unsigned i = 0; i < sizeof(vals) / sizeof(vals[0]); ++i) {
    printf("%s ", to_str(vals[i]));
  }
  printf("\n");
  return vals[0] + vals[1] + vals[2] + vals[3];
}
